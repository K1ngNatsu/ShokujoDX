//! The Library tab (P8 Milestone 2, ADR 0035): the designs in the library
//! folders as thumbnails or a list, with search, sort and group. Open a
//! design, make a new design from one, show it in File Explorer, print a
//! catalog (PDF) or save the design list (CSV).
//!
//! The index and the thumbnails are in the `library` folder next to the
//! settings file. The scan runs in a thread of its own, so the tab stays
//! usable; it starts when the program starts and when a folder is added.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};

use eframe::egui::{self, Color32, Rect, Sense, TextureHandle, Vec2, pos2, vec2};
use emb_library::{Group, GroupKey, Index, Query, ScanReport, SortKey};

use crate::a11y::Named;
use crate::app::AppState;
use crate::file::Notice;
use crate::i18n::tr;
use egui_phosphor::regular as ph;

/// The size of a thumbnail card, in screen points.
const CARD: Vec2 = Vec2::new(150.0, 176.0);
const THUMB: f32 = 128.0;
const HEADER_H: f32 = 26.0;
const LIST_ROW_H: f32 = 22.0;
/// Thumbnails read from disk in one frame (the rest come in the next frames).
const THUMBS_PER_FRAME: usize = 24;

/// A scan that runs in the background.
pub struct ScanJob {
    pub done: Arc<AtomicUsize>,
    pub total: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    rx: mpsc::Receiver<(Index, ScanReport)>,
}

/// The state of the Library tab.
#[derive(Default)]
pub struct LibraryUi {
    pub index: Index,
    /// The folder of the index file and the thumbnails (`None`: kept in
    /// memory only, with no thumbnails).
    pub dir: Option<PathBuf>,
    pub text: String,
    pub sort: SortKey,
    pub descending: bool,
    pub group: GroupKey,
    pub list_view: bool,
    /// The selected designs, by path, in the order of selection.
    pub selected: Vec<PathBuf>,
    anchor: Option<PathBuf>,
    /// The found entries, sorted, and their groups.
    pub found: Vec<usize>,
    pub groups: Vec<Group>,
    pub problems: Vec<String>,
    /// The search, sort or index changed: `found` must be made again.
    stale: bool,
    /// The machine of the hoop groups (they change with the machine).
    group_machine: Option<String>,
    pub scan: Option<ScanJob>,
    thumbs: HashMap<String, Option<TextureHandle>>,
    started: bool,
}

impl LibraryUi {
    fn index_file(&self) -> Option<PathBuf> {
        self.dir.as_ref().map(|d| d.join("index.json"))
    }

    fn thumb_dir(&self) -> Option<PathBuf> {
        self.dir.as_ref().map(|d| d.join("thumbnails"))
    }

    /// The found entries in the order that the tab shows them (group by group).
    pub fn shown(&self) -> Vec<usize> {
        self.groups.iter().flat_map(|g| g.items.iter().copied()).collect()
    }

    pub fn is_selected(&self, path: &Path) -> bool {
        self.selected.iter().any(|p| p == path)
    }
}

/// The library folder in the settings folder.
pub fn library_dir() -> Option<PathBuf> {
    crate::about::user_folder().map(|d| d.join("library"))
}

impl AppState {
    /// At the start: reads the index file and scans the library folders.
    pub fn start_library(&mut self, dir: Option<PathBuf>) {
        self.library.dir = dir;
        if let Some(f) = self.library.index_file() {
            self.library.index = Index::load(&f);
        }
        self.library.stale = true;
        self.library.started = true;
        if !self.settings.library_folders.is_empty() {
            self.scan_library();
        }
    }

    /// Adds a library folder and scans again.
    pub fn add_library_folder(&mut self, path: PathBuf) {
        if self.settings.library_folders.contains(&path) {
            self.message = crate::i18n::fill("{} is a library folder already.", &[&(path.display())]);
            return;
        }
        let mut s = self.settings.clone();
        s.library_folders.push(path);
        self.set_settings(s);
        self.scan_library();
    }

    /// Removes library folder `i` and scans again.
    pub fn remove_library_folder(&mut self, i: usize) {
        let mut s = self.settings.clone();
        if i < s.library_folders.len() {
            s.library_folders.remove(i);
            self.set_settings(s);
            self.scan_library();
        }
    }

    /// Starts a scan of the library folders in the background (a scan that
    /// runs stops first).
    pub fn scan_library(&mut self) {
        if let Some(job) = self.library.scan.take() {
            job.stop.store(true, Ordering::Relaxed);
        }
        let roots = self.settings.library_folders.clone();
        let old = self.library.index.clone();
        let thumbs = self.library.thumb_dir();
        let done = Arc::new(AtomicUsize::new(0));
        let total = Arc::new(AtomicUsize::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::channel();
        let (d, t, s) = (done.clone(), total.clone(), stop.clone());
        std::thread::spawn(move || {
            let result = emb_library::scan(&roots, &old, thumbs.as_deref(), |k, n| {
                d.store(k, Ordering::Relaxed);
                t.store(n, Ordering::Relaxed);
                !s.load(Ordering::Relaxed)
            });
            let _ = tx.send(result);
        });
        self.library.scan = Some(ScanJob { done, total, stop, rx });
    }

    /// Stops the scan; the designs read so far stay in the index.
    pub fn stop_library_scan(&mut self) {
        if let Some(job) = &self.library.scan {
            job.stop.store(true, Ordering::Relaxed);
        }
    }

    /// Takes the result of a scan that ended. `true` while one still runs
    /// (the tab paints again soon).
    pub fn poll_library(&mut self) -> bool {
        if let Some(job) = &self.library.scan {
            match job.rx.try_recv() {
                Ok((index, report)) => {
                    self.library.scan = None;
                    self.set_library_index(index);
                    self.message = crate::i18n::fill(
                        "Library: {} designs ({} read, {} removed{}){}",
                        &[
                            &(self.library.index.entries.len()),
                            &report.read,
                            &report.removed,
                            &(if report.failed > 0 {
                                format!(", {} cannot be read", report.failed)
                            } else {
                                String::new()
                            }),
                            &(if report.stopped { "; the scan was stopped" } else { "" }),
                        ],
                    );
                }
                Err(mpsc::TryRecvError::Disconnected) => self.library.scan = None,
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        self.library.scan.is_some()
    }

    /// Puts in a new index (and saves it); the selection keeps the designs
    /// that are still there.
    pub fn set_library_index(&mut self, index: Index) {
        let lib = &mut self.library;
        lib.selected.retain(|p| index.entry(p).is_some());
        // Thumbnails that are gone do not stay in memory.
        let names: std::collections::HashSet<&String> = index.entries.iter().filter_map(|e| e.thumb.as_ref()).collect();
        lib.thumbs.retain(|k, _| names.contains(k));
        lib.index = index;
        lib.stale = true;
        if let Some(f) = lib.index_file()
            && let Err(e) = lib.index.save(&f)
        {
            self.message = crate::i18n::fill("The library index is not saved: {}", &[&e]);
        }
    }

    /// Makes the found list again if the search, sort, group or index changed.
    pub fn refresh_library(&mut self) {
        let machine = self.doc.design().machine.clone();
        if self.library.group == GroupKey::Hoop && machine != self.library.group_machine {
            self.library.stale = true;
        }
        if !self.library.stale {
            return;
        }
        self.library.group_machine = machine;
        self.library.stale = false;
        let q = Query::parse(&self.library.text, &self.hoops);
        let lib = &mut self.library;
        lib.problems = q.problems();
        let mut found = q.run(&lib.index);
        emb_library::sort(&lib.index, &mut found, lib.sort, lib.descending);
        let hoops: Vec<emb_model::HoopPreset> = self.auto_hoop_candidates().into_iter().cloned().collect();
        let lib = &mut self.library;
        lib.groups = emb_library::group(&lib.index, &found, lib.group, &hoops);
        lib.found = found;
    }

    /// Sets the search text.
    pub fn set_library_search(&mut self, text: &str) {
        self.library.text = text.to_string();
        self.library.stale = true;
        self.refresh_library();
    }

    /// The entries of the selected designs, in index order.
    fn selected_entries(&self) -> Vec<&emb_library::Entry> {
        self.library
            .index
            .entries
            .iter()
            .filter(|e| self.library.is_selected(&e.path))
            .collect()
    }

    /// Opens the selected designs, each one in a tab of its own.
    pub fn open_library_selection(&mut self) {
        let paths: Vec<PathBuf> = self.selected_entries().iter().map(|e| e.path.clone()).collect();
        for p in paths {
            self.request(crate::file::Pending::Open(Some(p)));
        }
        self.focus_canvas = true;
    }

    /// New from selected: a new, unsaved design that is a copy of the
    /// first selected design (a project file, or the stitches of a machine
    /// file).
    pub fn new_from_library_selection(&mut self) {
        let Some(e) = self.selected_entries().first().map(|e| (*e).clone()) else {
            return;
        };
        let design = if e.format == emb_model::project::FILE_EXTENSION {
            match emb_model::project::load(&e.path) {
                Ok(d) => Some(d),
                Err(err) => {
                    self.notice = Some(Notice {
                        title: "Cannot read the design".into(),
                        lines: vec![e.path.display().to_string(), err.to_string()],
                    });
                    None
                }
            }
        } else {
            self.read_machine_file(&e.path, &e.format).map(|mut d| {
                d.machine = self.doc.design().machine.clone();
                d.hoop.selection = self.doc.design().hoop.selection.clone();
                d
            })
        };
        if let Some(mut d) = design {
            d.name = format!("{} (new)", e.name);
            self.start_design(d);
            self.focus_canvas = true;
            self.message = crate::i18n::fill("New design from {}. Save it to keep it.", &[&e.file]);
        }
    }

    /// Output selected: asks for a folder and queues the batch output (the
    /// files of the output profile) of the selected designs.
    pub fn output_library_selection(&mut self) {
        let paths: Vec<PathBuf> = self.selected_entries().iter().map(|e| e.path.clone()).collect();
        if paths.is_empty() {
            return;
        }
        let Some(folder) = rfd::FileDialog::new()
            .set_title("Output selected: folder for the files")
            .set_directory(self.dialog_dir())
            .pick_folder()
        else {
            return;
        };
        let (profile, machine) = (self.settings.output.clone(), self.doc.design().machine.clone());
        let n = paths.len();
        self.queue_batch(paths, folder, profile, machine);
        self.message = crate::i18n::fill(
            "Output of {} designs is in the queue (the Exports tab shows it).",
            &[&n],
        );
    }

    /// Shows the first selected design in File Explorer.
    pub fn show_library_selection(&mut self) {
        if let Some(p) = self.selected_entries().first().map(|e| e.path.clone()) {
            show_in_file_manager(&p);
        }
    }

    /// Asks for a file and queues the catalog of the found designs.
    pub fn library_catalog_dialog(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("Design catalog")
            .add_filter("PDF", &["pdf"])
            .set_file_name("Design catalog.pdf")
            .save_file()
        else {
            return;
        };
        let paths: Vec<PathBuf> = self
            .library
            .shown()
            .iter()
            .map(|&i| self.library.index.entries[i].path.clone())
            .collect();
        let title = self.catalog_title();
        let paper = self.settings.output.paper;
        self.queue_catalog(paths, title, paper, path);
        self.message = tr("The catalog is in the queue (the Exports tab shows it).").into();
    }

    fn catalog_title(&self) -> String {
        let date = chrono::Local::now().format("%Y-%m-%d");
        if self.library.text.trim().is_empty() {
            format!("Design catalog · {date}")
        } else {
            format!("Design catalog: {} · {date}", self.library.text.trim())
        }
    }

    /// Asks for a file and writes the list of the found designs (CSV).
    pub fn library_csv_dialog(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("Design list")
            .add_filter("CSV", &["csv"])
            .set_file_name("Design list.csv")
            .save_file()
        else {
            return;
        };
        let text = emb_library::csv::design_list(&self.library.index, &self.library.shown());
        self.message = match std::fs::write(&path, text) {
            Ok(()) => format!("Wrote {}", path.display()),
            Err(e) => format!("Cannot write {}: {e}", path.display()),
        };
    }

    /// A click on a design: select it (Ctrl: add or take away; Shift: all
    /// from the last click).
    pub fn click_library_design(&mut self, path: &Path, mods: egui::Modifiers) {
        let lib = &mut self.library;
        if mods.shift
            && let Some(anchor) = lib.anchor.clone()
        {
            let order: Vec<PathBuf> = lib.shown().iter().map(|&i| lib.index.entries[i].path.clone()).collect();
            let (a, b) = (
                order.iter().position(|p| *p == anchor),
                order.iter().position(|p| p == path),
            );
            if let (Some(a), Some(b)) = (a, b) {
                lib.selected = order[a.min(b)..=a.max(b)].to_vec();
                return;
            }
        }
        if mods.command || mods.ctrl {
            if let Some(k) = lib.selected.iter().position(|p| p == path) {
                lib.selected.remove(k);
            } else {
                lib.selected.push(path.to_path_buf());
            }
        } else {
            lib.selected = vec![path.to_path_buf()];
        }
        lib.anchor = Some(path.to_path_buf());
    }

    /// The texture of a thumbnail; read from disk on first use (a few in
    /// each frame).
    fn thumb_texture(&mut self, ctx: &egui::Context, name: &str, budget: &mut usize) -> Option<TextureHandle> {
        let dir = self.library.thumb_dir();
        thumb(&mut self.library.thumbs, ctx, dir.as_deref(), name, budget)
    }
}

/// The texture of the thumbnail `name` in `dir`, from the cache `map`; read
/// from disk on first use, while `budget` (thumbnails left for this frame)
/// is not 0.
pub fn thumb(
    map: &mut HashMap<String, Option<TextureHandle>>,
    ctx: &egui::Context,
    dir: Option<&Path>,
    name: &str,
    budget: &mut usize,
) -> Option<TextureHandle> {
    if let Some(t) = map.get(name) {
        return t.clone();
    }
    if *budget == 0 {
        return None;
    }
    *budget -= 1;
    let t = dir
        .and_then(|d| std::fs::read(d.join(name)).ok())
        .and_then(|b| crate::backdrop::decode(&b, crate::resources::max_texture_side(ctx)).ok())
        .map(|img| ctx.load_texture(format!("thumb-{name}"), img, egui::TextureOptions::LINEAR));
    map.insert(name.to_string(), t.clone());
    t
}

/// Writes the catalog PDF of `entries` (the stitches are read again from
/// the files).
pub fn write_catalog(
    entries: &[emb_library::Entry],
    title: &str,
    paper: emb_render::page::Paper,
    path: &Path,
) -> Result<(), String> {
    let items: Vec<emb_library::catalog::Item> = entries
        .iter()
        .map(|e| emb_library::catalog::Item {
            entry: e,
            stitches: e
                .error
                .is_none()
                .then(|| emb_library::read_stitches(&e.path).ok())
                .flatten(),
        })
        .collect();
    let pages = emb_library::catalog::pages(title, &items, paper);
    let pdf = emb_render::pdf::write(&pages, title);
    std::fs::write(path, pdf).map_err(|e| format!("{}: {e}", path.display()))
}

/// Opens the folder of `path` in the file manager, with the file selected
/// where the system can do that.
pub fn show_in_file_manager(path: &Path) {
    #[cfg(target_os = "windows")]
    let r = std::process::Command::new("explorer")
        .arg(format!("/select,{}", path.display()))
        .spawn();
    #[cfg(target_os = "macos")]
    let r = std::process::Command::new("open").arg("-R").arg(path).spawn();
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let r = std::process::Command::new("xdg-open")
        .arg(path.parent().unwrap_or(path))
        .spawn();
    let _ = r;
}

/// A short line about an entry: format and size (the colors are in the
/// hover box and the list).
fn summary(st: &AppState, e: &emb_library::Entry) -> String {
    let format = format_label(e);
    if e.error.is_some() {
        return format!("{format} · cannot be read");
    }
    let u = st.settings.units;
    let decimals = if u == crate::workspace::Units::Mm { 0 } else { 1 };
    format!(
        "{format} · {:.*} × {:.*} {}",
        decimals,
        u.of_mm(e.width_mm),
        decimals,
        u.of_mm(e.height_mm),
        u.suffix()
    )
}

/// "Project" for a project file, else the format in capitals.
fn format_label(e: &emb_library::Entry) -> String {
    if e.format == emb_model::project::FILE_EXTENSION {
        "Project".into()
    } else {
        e.format.to_uppercase()
    }
}

/// The text of the hover box of an entry.
fn details(st: &AppState, e: &emb_library::Entry) -> String {
    let u = st.settings.units;
    let mut lines = vec![e.name.clone(), e.path.display().to_string()];
    match &e.error {
        Some(err) => lines.push(format!("Cannot read the file: {err}")),
        None => {
            lines.push(format!("{} × {}", u.fmt(e.width_mm), u.fmt(e.height_mm)));
            lines.push(format!(
                "{} stitches, {} colors, {} trims",
                e.stitches, e.colors, e.trims
            ));
            if !e.threads.is_empty() {
                let names: Vec<&str> = e.threads.iter().map(|t| t.name.as_str()).collect();
                lines.push(format!("Threads: {}", names.join(", ")));
            }
        }
    }
    if let Some(m) = &e.machine {
        lines.push(format!("Machine: {m}"));
    }
    if !e.notes.is_empty() {
        lines.push(format!("Notes: {}", e.notes));
    }
    lines.join("\n")
}

/// A rows of the Library tab: a group title or designs.
enum Row {
    Header(String, usize),
    Cards(Vec<usize>),
    Line(usize),
}

/// The Library tab.
pub fn panel(ui: &mut egui::Ui, st: &mut AppState) {
    if !st.library.started {
        st.start_library(library_dir());
    }
    if st.poll_library() {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(150));
    }
    st.refresh_library();
    toolbar(ui, st);
    ui.separator();
    if st.settings.library_folders.is_empty() {
        ui.add_space(24.0);
        ui.vertical_centered(|ui| {
            ui.label(tr(
                "The library shows the designs in your design folders and their subfolders.",
            ));
            ui.add_space(6.0);
            if ui.button(tr("Add a library folder…")).clicked()
                && let Some(p) = rfd::FileDialog::new().set_title("Library folder").pick_folder()
            {
                st.add_library_folder(p);
            }
        });
        return;
    }
    body(ui, st);
}

fn toolbar(ui: &mut egui::Ui, st: &mut AppState) {
    ui.horizontal_wrapped(|ui| {
        ui.menu_button(format!("{}  Folders", ph::FOLDERS), |ui| {
            let folders = st.settings.library_folders.clone();
            let mut remove = None;
            for (i, f) in folders.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(f.display().to_string());
                    if ui
                        .small_button(tr("Remove"))
                        .on_hover_text(tr("Take the folder out of the library (no file is deleted)."))
                        .clicked()
                    {
                        remove = Some(i);
                    }
                });
            }
            if let Some(i) = remove {
                st.remove_library_folder(i);
            }
            if !folders.is_empty() {
                ui.separator();
            }
            if ui.button(tr("Add folder…")).clicked() {
                ui.close();
                if let Some(p) = rfd::FileDialog::new().set_title("Library folder").pick_folder() {
                    st.add_library_folder(p);
                }
            }
            if ui
                .button(tr("Scan again"))
                .on_hover_text(tr("Read the new and changed designs."))
                .clicked()
            {
                ui.close();
                st.scan_library();
            }
        });
        let mut text = st.library.text.clone();
        let edit = ui.add(
            egui::TextEdit::singleline(&mut text)
                .hint_text(tr("Search: words, format:vp3, colors:<4, width:<100, fits:crown"))
                .desired_width(340.0),
        );
        edit.on_hover_text(tr(
            "Each word must be in the name, file, folder, notes, threads or format.\n\
             Filters: format:vp3,dst  folder:caps  colors:<4  stitches:1000-5000\n\
             width:<100  height:>50  fits:<hoop>  error:yes  \"two words\"",
        ));
        if text != st.library.text {
            st.set_library_search(&text);
        }
        let (mut sort, mut desc, mut group, mut list) = (
            st.library.sort,
            st.library.descending,
            st.library.group,
            st.library.list_view,
        );
        egui::ComboBox::from_id_salt("library-sort")
            .selected_text(format!("Sort: {}", sort.label()))
            .show_ui(ui, |ui| {
                for k in SortKey::ALL {
                    ui.selectable_value(&mut sort, k, k.label());
                }
            });
        let arrow = if desc { ph::SORT_DESCENDING } else { ph::SORT_ASCENDING };
        if ui
            .button(arrow)
            .on_hover_text(if desc {
                tr("Largest first")
            } else {
                tr("Smallest first")
            })
            .named(if desc {
                tr("Largest first")
            } else {
                tr("Smallest first")
            })
            .clicked()
        {
            desc = !desc;
        }
        egui::ComboBox::from_id_salt("library-group")
            .selected_text(format!("Group: {}", group.label()))
            .show_ui(ui, |ui| {
                for k in GroupKey::ALL {
                    ui.selectable_value(&mut group, k, k.label());
                }
            });
        ui.selectable_value(&mut list, false, ph::SQUARES_FOUR)
            .on_hover_text(tr("Thumbnails"));
        ui.selectable_value(&mut list, true, ph::LIST).on_hover_text(tr("List"));
        if (sort, desc, group) != (st.library.sort, st.library.descending, st.library.group) {
            st.library.stale = true;
        }
        st.library.sort = sort;
        st.library.descending = desc;
        st.library.group = group;
        st.library.list_view = list;
        st.refresh_library();
    });
    ui.horizontal_wrapped(|ui| {
        let n = st.library.selected.len();
        if ui
            .add_enabled(n > 0, egui::Button::new(tr("Open")))
            .on_hover_text(tr("Open the selected designs (also: double-click)."))
            .clicked()
        {
            st.open_library_selection();
        }
        if ui
            .add_enabled(n > 0, egui::Button::new(tr("New from selected")))
            .on_hover_text(tr("A new design that is a copy of the selected design. The file does not change."))
            .clicked()
        {
            st.new_from_library_selection();
        }
        if ui
            .add_enabled(n > 0, egui::Button::new(tr("Show in File Explorer")))
            .clicked()
        {
            st.show_library_selection();
        }
        if ui
            .add_enabled(n > 0, egui::Button::new(tr("Output selected…")))
            .on_hover_text(tr("The files of the output profile (machine files, worksheets, pictures) for each selected design, in a folder."))
            .clicked()
        {
            st.output_library_selection();
        }
        if ui
            .add_enabled(n > 0, egui::Button::new(format!("{}  Send to USB", ph::USB)))
            .on_hover_text(
                tr("Send the selected designs to the USB stick of the machine of the open design, in its format."),
            )
            .clicked()
        {
            st.send_library_selection();
        }
        let any = !st.library.found.is_empty();
        if ui
            .add_enabled(any, egui::Button::new(tr("Catalog (PDF)…")))
            .on_hover_text(tr("Print the found designs: a picture of each one with its numbers, 20 on a page."))
            .clicked()
        {
            st.library_catalog_dialog();
        }
        if ui
            .add_enabled(any, egui::Button::new(tr("Design list (CSV)…")))
            .on_hover_text(tr("The found designs and their numbers, for a spreadsheet."))
            .clicked()
        {
            st.library_csv_dialog();
        }
        ui.separator();
        let total = st.library.index.entries.len();
        ui.weak(crate::i18n::fill("{} of {} designs{}", &[&(st.library.found.len()), &total, &(if n > 0 {
                format!(" · {n} selected")
            } else {
                String::new()
            })]));
        if let Some(job) = &st.library.scan {
            let (d, t) = (job.done.load(Ordering::Relaxed), job.total.load(Ordering::Relaxed));
            ui.add(egui::Spinner::new());
            ui.weak(if t > 0 {
                format!("Reading designs: {d} of {t}")
            } else {
                "Looking for designs…".into()
            });
            if ui.small_button(tr("Stop")).clicked() {
                st.stop_library_scan();
            }
        }
        for p in &st.library.problems {
            ui.colored_label(ui.visuals().warn_fg_color, p);
        }
    });
}

fn rows(st: &AppState, columns: usize) -> Vec<Row> {
    let mut out = Vec::new();
    let grouped = st.library.group != GroupKey::None;
    for g in &st.library.groups {
        if grouped {
            out.push(Row::Header(g.label.clone(), g.items.len()));
        }
        if st.library.list_view {
            out.extend(g.items.iter().map(|&i| Row::Line(i)));
        } else {
            out.extend(g.items.chunks(columns.max(1)).map(|c| Row::Cards(c.to_vec())));
        }
    }
    out
}

fn row_height(r: &Row) -> f32 {
    match r {
        Row::Header(..) => HEADER_H,
        Row::Cards(_) => CARD.y + 6.0,
        Row::Line(_) => LIST_ROW_H,
    }
}

fn body(ui: &mut egui::Ui, st: &mut AppState) {
    if st.library.found.is_empty() {
        ui.add_space(12.0);
        ui.weak(if st.library.index.entries.is_empty() {
            "No designs yet."
        } else {
            "No design matches the search."
        });
        return;
    }
    if st.library.list_view {
        list_header(ui, st);
    }
    let columns = ((ui.available_width() / (CARD.x + 6.0)).floor() as usize).max(1);
    let rows = rows(st, columns);
    let total: f32 = rows.iter().map(row_height).sum();
    let ctx = ui.ctx().clone();
    let mut budget = THUMBS_PER_FRAME;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_viewport(ui, |ui, viewport| {
            let width = ui.available_width();
            ui.set_height(total);
            let origin = ui.max_rect().min;
            let mut y = 0.0;
            for r in &rows {
                let h = row_height(r);
                if y + h >= viewport.min.y && y <= viewport.max.y {
                    let rect = Rect::from_min_size(origin + vec2(0.0, y), vec2(width, h));
                    match r {
                        Row::Header(label, n) => {
                            ui.painter().text(
                                rect.left_center() + vec2(4.0, 0.0),
                                egui::Align2::LEFT_CENTER,
                                format!("{label}  ({n})"),
                                egui::FontId::proportional(14.0),
                                ui.visuals().strong_text_color(),
                            );
                        }
                        Row::Cards(items) => {
                            for (k, &i) in items.iter().enumerate() {
                                let r = Rect::from_min_size(rect.min + vec2(k as f32 * (CARD.x + 6.0), 3.0), CARD);
                                card(ui, st, &ctx, i, r, &mut budget);
                            }
                        }
                        Row::Line(i) => line(ui, st, *i, rect),
                    }
                }
                y += h;
            }
        });
    if budget == 0 {
        // More thumbnails to read.
        ctx.request_repaint();
    }
}

/// Clicks, double-clicks, the hover box and the context menu of a design.
fn interact(ui: &mut egui::Ui, st: &mut AppState, i: usize, rect: Rect) -> bool {
    let e = st.library.index.entries[i].clone();
    let resp = ui
        .interact(rect, egui::Id::new(("library-design", &e.path)), Sense::click())
        .on_hover_text(details(st, &e));
    if resp.clicked() {
        let mods = ui.input(|i| i.modifiers);
        st.click_library_design(&e.path, mods);
    }
    if resp.double_clicked() {
        st.library.selected = vec![e.path.clone()];
        st.open_library_selection();
    }
    resp.context_menu(|ui| {
        if !st.library.is_selected(&e.path) {
            st.library.selected = vec![e.path.clone()];
        }
        if ui.button(tr("Open")).clicked() {
            st.open_library_selection();
            ui.close();
        }
        if ui.button(tr("New from selected")).clicked() {
            st.new_from_library_selection();
            ui.close();
        }
        if ui.button(tr("Show in File Explorer")).clicked() {
            st.show_library_selection();
            ui.close();
        }
        if ui.button(tr("Send to USB")).clicked() {
            st.send_library_selection();
            ui.close();
        }
    });
    resp.hovered()
}

fn card(ui: &mut egui::Ui, st: &mut AppState, ctx: &egui::Context, i: usize, rect: Rect, budget: &mut usize) {
    let hovered = interact(ui, st, i, rect);
    let e = st.library.index.entries[i].clone();
    let selected = st.library.is_selected(&e.path);
    let v = ui.visuals().clone();
    let fill = if selected {
        v.selection.bg_fill
    } else if hovered {
        v.widgets.hovered.weak_bg_fill
    } else {
        v.faint_bg_color
    };
    let p = ui.painter();
    p.rect_filled(rect, 6.0, fill);
    let pic = Rect::from_center_size(
        pos2(rect.center().x, rect.min.y + 8.0 + THUMB / 2.0),
        Vec2::splat(THUMB),
    );
    let tex = e.thumb.as_ref().and_then(|t| st.thumb_texture(ctx, t, budget));
    let p = ui.painter();
    match tex {
        Some(t) => {
            let s = t.size_vec2();
            let k = (THUMB / s.x.max(s.y)).min(1.0);
            p.image(
                t.id(),
                Rect::from_center_size(pic.center(), s * k),
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        }
        None => {
            let why = if e.error.is_some() { "Cannot read" } else { "…" };
            p.text(
                pic.center(),
                egui::Align2::CENTER_CENTER,
                why,
                egui::FontId::proportional(12.0),
                if e.error.is_some() {
                    v.error_fg_color
                } else {
                    v.weak_text_color()
                },
            );
        }
    }
    let text_w = rect.width() - 10.0;
    let name = truncated(p, &e.name, 13.0, v.strong_text_color(), text_w);
    p.galley(pos2(rect.min.x + 5.0, pic.max.y + 6.0), name, v.text_color());
    let sub = truncated(p, &summary(st, &e), 11.0, v.weak_text_color(), text_w);
    p.galley(pos2(rect.min.x + 5.0, pic.max.y + 24.0), sub, v.text_color());
}

fn truncated(p: &egui::Painter, text: &str, size: f32, color: Color32, width: f32) -> Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::single_section(
        text.to_string(),
        egui::TextFormat {
            font_id: egui::FontId::proportional(size),
            color,
            ..Default::default()
        },
    );
    job.wrap = egui::text::TextWrapping::truncate_at_width(width);
    p.layout_job(job)
}

/// The columns of the list: title, sort key and left edge (points).
pub(crate) const LIST_COLUMNS: [(&str, SortKey, f32); 6] = [
    ("Name", SortKey::Name, 6.0),
    ("Format", SortKey::Format, 260.0),
    ("Size", SortKey::Size, 330.0),
    ("Stitches", SortKey::Stitches, 470.0),
    ("Colors", SortKey::Colors, 550.0),
    ("Folder", SortKey::Folder, 610.0),
];

fn list_header(ui: &mut egui::Ui, st: &mut AppState) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), LIST_ROW_H), Sense::hover());
    for (title, key, x) in LIST_COLUMNS {
        let mark = if st.library.sort == key {
            if st.library.descending { " ▼" } else { " ▲" }
        } else {
            ""
        };
        let r = Rect::from_min_size(rect.min + vec2(x - 4.0, 0.0), vec2(70.0, LIST_ROW_H));
        let resp = ui.interact(r, egui::Id::new(("library-col", title)), Sense::click());
        ui.painter().text(
            rect.min + vec2(x, LIST_ROW_H / 2.0),
            egui::Align2::LEFT_CENTER,
            format!("{}{mark}", tr(title)),
            egui::FontId::proportional(13.0),
            ui.visuals().strong_text_color(),
        );
        if resp
            .on_hover_text(tr("Sort by this column (again: the other way)"))
            .clicked()
        {
            if st.library.sort == key {
                st.library.descending = !st.library.descending;
            } else {
                st.library.sort = key;
                st.library.descending = false;
            }
            st.library.stale = true;
            st.refresh_library();
        }
    }
}

fn line(ui: &mut egui::Ui, st: &mut AppState, i: usize, rect: Rect) {
    let hovered = interact(ui, st, i, rect);
    let e = st.library.index.entries[i].clone();
    let v = ui.visuals().clone();
    let p = ui.painter();
    if st.library.is_selected(&e.path) {
        p.rect_filled(rect, 2.0, v.selection.bg_fill);
    } else if hovered {
        p.rect_filled(rect, 2.0, v.widgets.hovered.weak_bg_fill);
    }
    let u = st.settings.units;
    let cells = [
        e.name.clone(),
        format_label(&e),
        if e.error.is_some() {
            "cannot be read".into()
        } else {
            format!("{} × {}", u.fmt(e.width_mm), u.fmt(e.height_mm))
        },
        e.stitches.to_string(),
        e.colors.to_string(),
        e.folder.clone(),
    ];
    for (k, text) in cells.iter().enumerate() {
        let x = LIST_COLUMNS[k].2;
        let next = LIST_COLUMNS.get(k + 1).map_or(rect.width(), |c| c.2);
        let color = if k == 0 { v.strong_text_color() } else { v.text_color() };
        let g = truncated(p, text, 13.0, color, (next - x - 8.0).max(20.0));
        p.galley(rect.min + vec2(x, (LIST_ROW_H - g.size().y) / 2.0), g, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::Harness;
    use eframe::egui::{Event, Modifiers, PointerButton, RawInput};

    /// A library folder with three designs, scanned (and waited for).
    fn library(name: &str) -> (Harness, PathBuf) {
        let root = std::env::temp_dir().join(format!("emb-app-library-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let design = |name: &str, w: f64| {
            let mut d = emb_model::Design::new(name);
            let t = d.add_thread("Navy", emb_model::Rgb(20, 30, 90));
            d.add_object(
                t,
                emb_model::ObjectKind::Running {
                    path: emb_geom::Path::new()
                        .move_to(emb_geom::Point::new(0.0, 0.0))
                        .line_to(emb_geom::Point::new(w, 10.0)),
                    params: Default::default(),
                },
            );
            d
        };
        std::fs::create_dir_all(root.join("caps")).unwrap();
        emb_model::project::save(&design("Rose", 40.0), &root.join("rose.embproj")).unwrap();
        emb_model::project::save(&design("Eagle", 120.0), &root.join("caps/eagle.embproj")).unwrap();
        let mut h = Harness::empty();
        h.st.start_library(Some(root.join(".lib")));
        let mut s = h.st.settings.clone();
        s.library_folders = vec![root.clone()];
        h.st.settings = s;
        h.st.scan_library();
        while h.st.poll_library() {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        h.st.refresh_library();
        (h, root)
    }

    fn names(st: &AppState) -> Vec<String> {
        st.library
            .shown()
            .iter()
            .map(|&i| st.library.index.entries[i].name.clone())
            .collect()
    }

    #[test]
    fn scan_search_sort_group_and_select() {
        let (mut h, root) = library("search");
        assert_eq!(names(&h.st), ["Eagle", "Rose"]);
        assert!(root.join(".lib/index.json").is_file(), "the index is saved");
        assert_eq!(
            std::fs::read_dir(root.join(".lib/thumbnails")).unwrap().count(),
            2,
            "a thumbnail for each design"
        );
        h.st.set_library_search("caps");
        assert_eq!(names(&h.st), ["Eagle"]);
        h.st.set_library_search("width:<100");
        assert_eq!(names(&h.st), ["Rose"]);
        h.st.set_library_search("colors:many");
        assert!(names(&h.st).is_empty() && h.st.library.problems.len() == 1);
        h.st.set_library_search("");
        h.st.library.sort = SortKey::Size;
        h.st.library.descending = false;
        h.st.library.group = GroupKey::Folder;
        h.st.library.stale = true;
        h.st.refresh_library();
        let labels: Vec<&str> = h.st.library.groups.iter().map(|g| g.label.as_str()).collect();
        assert_eq!(labels.len(), 2);
        assert!(labels[1].ends_with("/caps"), "{labels:?}");
        // Click, Ctrl+click, Shift+click.
        let rose = root.join("rose.embproj");
        let eagle = root.join("caps/eagle.embproj");
        h.st.click_library_design(&rose, Modifiers::NONE);
        h.st.click_library_design(&eagle, Modifiers::CTRL);
        assert_eq!(h.st.library.selected.len(), 2);
        h.st.click_library_design(&eagle, Modifiers::CTRL);
        assert_eq!(h.st.library.selected, std::slice::from_ref(&rose));
        h.st.click_library_design(&rose, Modifiers::NONE);
        h.st.click_library_design(&eagle, Modifiers::SHIFT);
        assert_eq!(h.st.library.selected.len(), 2, "all from the last click");
        // A second start reads the index file and keeps the entries.
        let mut h2 = Harness::empty();
        h2.st.start_library(Some(root.join(".lib")));
        assert_eq!(h2.st.library.index.entries.len(), 2);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn open_new_from_selected_and_the_catalog() {
        let (mut h, root) = library("open");
        let rose = root.join("rose.embproj");
        h.st.click_library_design(&rose, Modifiers::NONE);
        h.st.new_from_library_selection();
        assert_eq!(h.st.doc.design().name, "Rose (new)");
        assert!(h.st.file_path.is_none(), "the new design has no file yet");
        let tabs = h.st.tab_count();
        h.st.open_library_selection();
        assert_eq!(h.st.file_path.as_deref(), Some(rose.as_path()));
        assert_eq!(h.st.tab_count(), tabs + 1, "a tab of its own");
        let entries: Vec<_> = h.st.library.index.entries.clone();
        let pdf = root.join("catalog.pdf");
        write_catalog(&entries, "Test catalog", emb_render::page::Paper::A4, &pdf).unwrap();
        let bytes = std::fs::read(&pdf).unwrap();
        assert!(bytes.starts_with(b"%PDF"));
        let csv = emb_library::csv::design_list(&h.st.library.index, &h.st.library.shown());
        assert_eq!(csv.lines().count(), 3);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn the_tab_draws_cards_and_a_double_click_opens() {
        let (mut h, root) = library("draw");
        let ctx = egui::Context::default();
        let frame = |st: &mut AppState, events: Vec<Event>| {
            let input = RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(1000.0, 700.0))),
                events,
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
                egui::CentralPanel::default().show(ui, |ui| panel(ui, st));
            });
            out.textures_delta.clear();
        };
        frame(&mut h.st, vec![]);
        frame(&mut h.st, vec![]);
        assert_eq!(h.st.library.thumbs.len(), 2, "both thumbnails are read");
        // The first card is at the top left, under the two tool rows.
        let at = pos2(80.0, 140.0);
        let ev = |pressed| Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        frame(&mut h.st, vec![Event::PointerMoved(at)]);
        frame(&mut h.st, vec![ev(true)]);
        frame(&mut h.st, vec![ev(false)]);
        assert_eq!(h.st.library.selected.len(), 1);
        frame(&mut h.st, vec![ev(true)]);
        frame(&mut h.st, vec![ev(false)]);
        assert_eq!(
            h.st.file_path.as_deref(),
            Some(root.join("caps/eagle.embproj").as_path())
        );
        // The list view draws too.
        h.st.library.list_view = true;
        frame(&mut h.st, vec![]);
        std::fs::remove_dir_all(&root).ok();
    }
}
