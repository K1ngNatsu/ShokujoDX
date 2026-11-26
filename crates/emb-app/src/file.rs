//! New, Open, Save, Save As, artwork import, and the unsaved-changes prompt.
//!
//! An action that would discard changes (New, Open, closing the window)
//! asks first: Save, Don't save, or Cancel.

use std::path::{Path, PathBuf};

use eframe::egui;
use emb_geom::{Point, Transform};
use emb_model::{Design, HoopSelection, ObjectId, Rgb, ThreadId, project};

use crate::app::AppState;
use crate::i18n::tr;
use crate::vectors::ArtworkImport;

/// The length of the list of recent designs.
pub const MAX_RECENT: usize = 8;

/// An action that waits for the unsaved-changes prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pending {
    New,
    /// Close the active design tab.
    CloseTab,
    /// Close all design tabs; each unsaved design asks first.
    CloseAll,
    /// A new design from a template file, or from the built-in template (`None`).
    NewFrom(Option<PathBuf>),
    /// A new design from a built-in lace template (`emb_model::lace::TEMPLATES`).
    NewLace(usize),
    Open(Option<PathBuf>),
    Quit,
}

/// A message window with a title and lines (errors, import notes).
#[derive(Debug, Clone)]
pub struct Notice {
    pub title: String,
    pub lines: Vec<String>,
}

/// The name of a design as the program shows it: a new design with no
/// name of its own ("Untitled") shows in the language of the program.
pub fn shown_name(name: &str) -> String {
    if name == "Untitled" {
        crate::i18n::tr("Untitled").to_string()
    } else {
        name.to_string()
    }
}

impl AppState {
    /// Window title: file name, a mark if changed, and the app name.
    pub fn window_title(&self) -> String {
        let name = self
            .file_path
            .as_deref()
            .and_then(Path::file_name)
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| crate::i18n::fill("{} (not saved)", &[&shown_name(&self.doc.design().name)]));
        let mark = if self.doc.is_modified() { "● " } else { "" };
        format!("{mark}{name} — ShokujoDX {}", crate::about::VERSION)
    }

    /// Runs `action` now, or asks first if it closes a design with unsaved
    /// changes. New and Open use a new tab, so they never ask. Quit asks
    /// for each unsaved design in turn.
    pub fn request(&mut self, action: Pending) {
        let unsaved = self.doc.is_modified() && !self.doc.design().objects().is_empty();
        match action {
            Pending::Quit => {
                // The tabs come back at the next start: no question (ADR 0175).
                if self.settings.restore_tabs && self.backups.root.is_some() {
                    match self.backups_keep_tabs() {
                        Ok(()) => {
                            self.close_now = true;
                            return;
                        }
                        Err(e) => {
                            self.message = crate::i18n::fill("The open tabs were not kept: {}", &[&e]);
                        }
                    }
                }
                self.quit_queue = self.unsaved_tabs();
                match self.quit_queue.first() {
                    Some(&i) => {
                        self.switch_tab(i);
                        self.confirm = Some(Pending::Quit);
                    }
                    None => self.perform(Pending::Quit),
                }
            }
            Pending::CloseTab if unsaved => self.confirm = Some(action),
            Pending::CloseAll => self.close_all_step(),
            _ => self.perform(action),
        }
    }

    pub(crate) fn perform(&mut self, action: Pending) {
        match action {
            Pending::CloseTab => self.close_tab(),
            Pending::CloseAll => {
                // The user saved this design or said not to save it.
                self.close_tab();
                self.close_all_step();
            }
            Pending::New => self.new_design(),
            Pending::NewFrom(path) => self.new_from_template(path.as_deref()),
            Pending::NewLace(i) => self.new_from_lace(i),
            Pending::Open(path) => {
                // The browser trial: File > Open is an upload.
                #[cfg(target_arch = "wasm32")]
                if path.is_none() {
                    crate::web::upload();
                    return;
                }
                let path = path.or_else(|| {
                    let mut all = vec![project::FILE_EXTENSION];
                    all.extend(emb_formats::READ_EXTENSIONS);
                    all.push("svg");
                    crate::dialog::FileDialog::new()
                        .set_title(tr("Open design"))
                        .add_filter(tr("Designs and machine files"), &all)
                        .add_filter(tr("Embroidery project"), &[project::FILE_EXTENSION])
                        .add_filter(tr("Machine files"), &emb_formats::READ_EXTENSIONS)
                        .add_filter(tr("Ink/Stitch design (SVG)"), &["svg"])
                        .set_directory(self.dialog_dir())
                        .pick_file()
                });
                if let Some(p) = path {
                    self.open_path(&p);
                }
            }
            Pending::Quit => {
                let done = self.active_tab;
                self.quit_queue.retain(|i| *i != done);
                match self.quit_queue.first() {
                    Some(&i) => {
                        self.switch_tab(i);
                        self.confirm = Some(Pending::Quit);
                    }
                    None => {
                        self.close_now = true;
                        self.backups_clean_exit();
                    }
                }
            }
        }
    }

    /// Close all: asks for the next unsaved design, or closes all designs
    /// when none is left. The last tab gets a new empty design.
    fn close_all_step(&mut self) {
        if let Some(&i) = self.unsaved_tabs().first() {
            self.switch_tab(i);
            self.confirm = Some(Pending::CloseAll);
            return;
        }
        while self.tab_count() > 1 {
            self.close_tab();
        }
        if !self.is_blank() {
            self.close_tab();
        }
    }

    pub(crate) fn dialog_dir(&self) -> PathBuf {
        self.file_path
            .as_deref()
            .and_then(Path::parent)
            .map(Path::to_path_buf)
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_default()
    }

    /// A new empty design with the current machine and hoop.
    /// A new design from the template for New (see `Settings::template`),
    /// or with the built-in settings.
    pub fn new_design(&mut self) {
        let path = self.settings.template.as_ref().and_then(|name| {
            let dir = crate::styles::templates_dir()?;
            let p = dir.join(format!("{name}.{}", emb_model::template::FILE_EXTENSION));
            p.is_file().then_some(p)
        });
        match path {
            Some(p) => self.new_from_template(Some(&p)),
            None => self.new_builtin_design(),
        }
    }

    /// A new design with the built-in settings, on the machine and hoop of the current design.
    /// A new design from the built-in lace template `i`, on the machine and
    /// in the hoop of the design before.
    pub fn new_from_lace(&mut self, i: usize) {
        let Some((name, make)) = emb_model::lace::TEMPLATES.get(i) else {
            return;
        };
        let mut d = make();
        d.name = "Untitled".into();
        d.machine = self.doc.design().machine.clone();
        d.hoop.selection = self.doc.design().hoop.selection.clone();
        self.start_design(d);
        self.message = crate::i18n::fill("New design from template \"{}\"", &[&tr(name)]);
    }

    pub fn new_builtin_design(&mut self) {
        let mut d = Design::new("Untitled");
        d.machine = self.doc.design().machine.clone();
        d.hoop.selection = self.doc.design().hoop.selection.clone();
        self.start_design(d);
        self.message = tr("New design").into();
    }

    /// Opens a file of any kind that the program reads (from the command
    /// line, a double-click in the file manager or a drop on the window):
    /// artwork is imported into the design, a picture becomes the backdrop,
    /// a project, a machine file or an Ink/Stitch file opens in a new tab.
    pub fn open_any(&mut self, p: &Path) {
        let ext = p.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase());
        match ext.as_deref() {
            Some("svg" | "pdf" | "ai" | "eps" | "ps" | "emf" | "wmf" | "cdr") => match std::fs::read(p) {
                Ok(b) if emb_formats::inkstitch::is_inkstitch(&b) => self.open_path(p),
                Ok(b) => self.import_svg_bytes(&b, &p.display().to_string()),
                Err(e) => self.message = crate::i18n::fill("Cannot read {}: {}", &[&(p.display()), &e]),
            },
            // A DXF drawing becomes a sketch (ADR 0079).
            Some("dxf") => match std::fs::read(p) {
                Ok(b) => self.import_dxf_bytes(&b, &p.display().to_string()),
                Err(e) => self.message = crate::i18n::fill("Cannot read {}: {}", &[&(p.display()), &e]),
            },
            // A picture becomes the backdrop.
            Some(e) if crate::backdrop::FILE_TYPES.contains(&e) => match std::fs::read(p) {
                Ok(b) => self.import_bitmap_bytes(b, &p.display().to_string()),
                Err(e) => self.message = crate::i18n::fill("Cannot read {}: {}", &[&(p.display()), &e]),
            },
            _ => self.open_path(p),
        }
    }

    pub fn open_path(&mut self, path: &Path) {
        if let Some(i) = self.tab_of_file(path) {
            self.switch_tab(i);
            self.message = crate::i18n::fill("{} is open already", &[&(path.display())]);
            return;
        }
        if let Some(ext) = machine_ext(path) {
            self.open_machine_file(path, &ext);
            return;
        }
        if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("svg")) {
            self.open_svg_design(path);
            return;
        }
        match project::load(path) {
            Ok(design) => {
                let mut notes = Vec::new();
                if let HoopSelection::Preset { id } = &design.hoop.selection
                    && self.hoops.get(id).is_none()
                {
                    notes.push(format!(
                        "Hoop \"{id}\" is not in the hoop list. The canvas shows no hoop."
                    ));
                }
                if let Some(m) = &design.machine
                    && self.machines.get(m).is_none()
                {
                    notes.push(format!("Machine \"{m}\" is not in the machine list."));
                }
                self.start_design(design);
                self.file_path = Some(path.to_path_buf());
                self.file_known(path);
                self.add_recent(path);
                self.message = crate::i18n::fill("Opened {}", &[&(path.display())]);
                if !notes.is_empty() {
                    self.notice = Some(Notice {
                        title: "Opened with notes".into(),
                        lines: notes,
                    });
                }
            }
            Err(e) => {
                self.notice = Some(Notice {
                    title: "Cannot open design".into(),
                    lines: vec![path.display().to_string(), e.to_string()],
                });
            }
        }
    }

    /// Puts `path` first in the list of recent designs.
    pub fn add_recent(&mut self, path: &Path) {
        let mut s = self.settings.clone();
        s.recent.retain(|p| p != path);
        s.recent.insert(0, path.to_path_buf());
        s.recent.truncate(MAX_RECENT);
        self.set_settings(s);
    }

    /// File > Recent designs: opens `path`, or takes it off the list if
    /// the file is not there any more.
    pub fn open_recent(&mut self, path: &Path) {
        if path.is_file() {
            self.request(Pending::Open(Some(path.to_path_buf())));
        } else {
            let mut s = self.settings.clone();
            s.recent.retain(|p| p != path);
            self.set_settings(s);
            self.message = crate::i18n::fill("{} is not there any more.", &[&(path.display())]);
        }
    }

    /// Reads a machine file as a design of stitch objects. `None` (with a
    /// notice) on an error.
    pub(crate) fn read_machine_file(&mut self, path: &Path, ext: &str) -> Option<Design> {
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Design".into());
        let read = read_stitches(path);
        let problem = match read {
            Ok(s) if s.stitch_count() > 0 => {
                let mut d = emb_model::design_from_stitches(&s, &stem);
                // TBF and U01 files have the needle of each color: keep them
                // as pins (ADR 0063).
                let needles = crate::dialog::read(path)
                    .map(|b| emb_formats::read_needles(ext, &b))
                    .unwrap_or_default();
                d.pin_block_needles(&needles);
                return Some(d);
            }
            Ok(_) => tr("The file has no stitches.").to_string(),
            Err(e) => e,
        };
        self.notice = Some(Notice {
            title: "Cannot open machine file".into(),
            lines: vec![path.display().to_string(), problem],
        });
        None
    }

    /// Opens a machine file as a new design: one stitch object per color.
    /// The design has no project file until the user saves it.
    pub fn open_machine_file(&mut self, path: &Path, ext: &str) {
        let Some(mut d) = self.read_machine_file(path, ext) else {
            return;
        };
        d.machine = self.doc.design().machine.clone();
        d.hoop.selection = self.doc.design().hoop.selection.clone();
        let n = d.objects().len();
        let recognized = self
            .settings
            .recognize_on_open
            .then(|| crate::recognize_objects::recognize_all(&mut d));
        self.start_design(d);
        self.add_recent(path);
        self.message = match recognized {
            Some((made, with_type)) => crate::i18n::fill(
                "Opened {} as {} objects ({} with a stitch type). Save it to keep it as a project.",
                &[&(path.display()), &made, &with_type],
            ),
            None => crate::i18n::fill(
                "Opened {} as {} stitch objects. Save it to keep it as a project.",
                &[&(path.display()), &n],
            ),
        };
    }

    /// Opens an SVG file as a new design: an Ink/Stitch file with its
    /// objects and their settings (ADR 0135), other artwork as fills and
    /// lines. The design has no project file until the user saves it.
    pub fn open_svg_design(&mut self, path: &Path) {
        let read = std::fs::read(path).map_err(|e| e.to_string()).and_then(|b| {
            let inkstitch = emb_formats::inkstitch::is_inkstitch(&b);
            emb_formats::svg::import_svg(&b)
                .map(|i| (i, inkstitch))
                .map_err(|e| e.to_string())
        });
        let (imported, inkstitch) = match read {
            Ok(r) => r,
            Err(e) => {
                self.notice = Some(Notice {
                    title: "Cannot open design".into(),
                    lines: vec![path.display().to_string(), e],
                });
                return;
            }
        };
        if imported.objects.is_empty() {
            self.notice = Some(Notice {
                title: "Nothing to import".into(),
                lines: std::iter::once(tr("The file has no filled or stroked shapes.").to_string())
                    .chain(imported.warnings)
                    .collect(),
            });
            return;
        }
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Design".into());
        let mut d = Design::new(&stem);
        d.machine = self.doc.design().machine.clone();
        d.hoop.selection = self.doc.design().hoop.selection.clone();
        let shift = imported
            .bbox()
            .map(|b| d.hoop.center - b.center())
            .unwrap_or(Point::ZERO);
        let n = imported.add_to(&mut d, &Transform::translate(shift.x, shift.y)).len();
        self.start_design(d);
        self.add_recent(path);
        self.fit_requested = true;
        self.message = if inkstitch {
            crate::i18n::fill(
                "Opened the Ink/Stitch file {} as {} objects. Save it to keep it as a project.",
                &[&(path.display()), &n],
            )
        } else {
            crate::i18n::fill(
                "Opened {} as {} objects. Save it to keep it as a project.",
                &[&(path.display()), &n],
            )
        };
        if !imported.warnings.is_empty() {
            self.notice = Some(Notice {
                title: "Opened with notes".into(),
                lines: imported.warnings,
            });
        }
    }

    /// Asks for a machine file and adds its stitch objects to the design.
    pub fn insert_machine_file_dialog(&mut self) {
        let dialog = crate::dialog::FileDialog::new()
            .set_title(tr("Insert machine file"))
            .add_filter(tr("Machine files"), &emb_formats::READ_EXTENSIONS)
            .set_directory(self.dialog_dir());
        crate::dialog::open_then(dialog, self, |st, p| {
            if let Some(ext) = machine_ext(&p) {
                st.insert_machine_file(&p, &ext);
            }
        });
    }

    /// Adds the stitch objects of a machine file to the design, as one
    /// undo step. Threads of the same color are shared.
    pub fn insert_machine_file(&mut self, path: &Path, ext: &str) {
        let Some(src) = self.read_machine_file(path, ext) else {
            return;
        };
        let c = self.doc.design().hoop.center;
        let ids = self.doc.edit(tr("Insert machine file"), |d| {
            let mut ids = Vec::new();
            for o in src.objects() {
                let Some(t) = src.thread(o.thread) else { continue };
                let thread = thread_for_color(d, t.color);
                let mut kind = o.kind.clone();
                // A machine file has its origin at the hoop center.
                kind.transform(&Transform::translate(c.x, c.y));
                let id = d.add_object(thread, kind);
                if let Some(n) = d.object_mut(id) {
                    n.functions = o.functions;
                }
                ids.push(id);
            }
            ids
        });
        self.message = crate::i18n::fill("Inserted {} stitch objects from {}", &[&(ids.len()), &(path.display())]);
        self.selection.set_all(ids);
    }

    /// Saves to the current file, or asks for a name. Returns `true` if saved.
    pub fn save(&mut self) -> bool {
        match self.file_path.clone() {
            Some(p) => self.save_to(&p),
            None => self.save_as(),
        }
    }

    /// Asks for a file name and saves. Returns `true` if saved.
    pub fn save_as(&mut self) -> bool {
        // The browser trial: Save is a download.
        #[cfg(target_arch = "wasm32")]
        {
            return self.download_project();
        }
        #[cfg_attr(target_arch = "wasm32", allow(unreachable_code))]
        let name = format!("{}.{}", self.doc.design().name, project::FILE_EXTENSION);
        let Some(mut p) = crate::dialog::FileDialog::new()
            .set_title(tr("Save design as"))
            .add_filter(tr("Embroidery project"), &[project::FILE_EXTENSION])
            .set_directory(self.dialog_dir())
            .set_file_name(name)
            .save_file()
        else {
            return false;
        };
        if p.extension().is_none_or(|e| e != project::FILE_EXTENSION) {
            p.set_extension(project::FILE_EXTENSION);
        }
        self.save_to(&p)
    }

    /// Shows a notice if the document did not make the last change: the
    /// change put an object out of the range of a project file (ADR 0139).
    pub fn show_refused_change(&mut self) {
        if let Some(why) = self.doc.take_refused() {
            self.notice = Some(Notice {
                title: "The change was not made".into(),
                lines: vec![
                    why,
                    tr("All parts of a design must be less than 1 km from its center.").to_string(),
                ],
            });
        }
    }

    /// Writes the project file. The design name follows the file name.
    pub fn save_to(&mut self, path: &Path) -> bool {
        if let Some(stem) = path.file_stem().map(|s| s.to_string_lossy().into_owned())
            && stem != self.doc.design().name
        {
            self.doc.edit(tr("Rename design"), |d| d.name = stem);
        }
        match project::save(self.doc.design(), path) {
            Ok(()) => {
                self.doc.mark_saved();
                self.file_path = Some(path.to_path_buf());
                self.file_known(path);
                self.add_recent(path);
                self.message = crate::i18n::fill("Saved {}", &[&(path.display())]);
                true
            }
            Err(e) => {
                self.notice = Some(Notice {
                    title: "Cannot save design".into(),
                    lines: vec![path.display().to_string(), e.to_string()],
                });
                false
            }
        }
    }

    /// Asks for an SVG file and imports it: as fills and lines, or (`lines`)
    /// every shape as a running-stitch line.
    pub fn import_artwork_dialog(&mut self, mode: ArtworkImport) {
        // In the browser the artwork comes as an upload (the owner,
        // 2026-11-25), as the picture of Picture to Design.
        let dialog = crate::dialog::FileDialog::new()
            .set_title(match mode {
                ArtworkImport::Objects => tr("Import Artwork"),
                ArtworkImport::Lines => tr("Import artwork as lines"),
                ArtworkImport::Vectors => tr("Import artwork as vectors"),
            })
            .add_filter(
                tr("Vector artwork (SVG, PDF, AI, EPS, EMF, WMF)"),
                &emb_formats::artwork::ARTWORK_EXTENSIONS,
            )
            .add_filter(tr("DXF drawing (opens as a sketch)"), &["dxf"])
            .set_directory(self.dialog_dir());
        crate::dialog::open_then(dialog, self, move |st, p| st.import_artwork_file(&p, mode));
    }

    /// Imports the artwork file `p` in the way of `mode` (a DXF drawing
    /// opens as a sketch).
    pub fn import_artwork_file(&mut self, p: &Path, mode: ArtworkImport) {
        let dxf = p.extension().is_some_and(|e| e.eq_ignore_ascii_case("dxf"));
        match crate::dialog::read(p) {
            Ok(bytes) if dxf => self.import_dxf_bytes(&bytes, &p.display().to_string()),
            Ok(bytes) => self.import_svg_as(&bytes, &p.display().to_string(), mode),
            Err(e) => {
                self.notice = Some(Notice {
                    title: "Cannot read file".into(),
                    lines: vec![p.display().to_string(), e.to_string()],
                })
            }
        }
    }

    /// Imports SVG data as objects, centered in the hoop, as one undo step.
    /// Colors become threads (an existing thread with the same color is reused).
    pub fn import_svg_bytes(&mut self, bytes: &[u8], source: &str) {
        self.import_svg_as(bytes, source, ArtworkImport::Objects);
    }

    /// As [`AppState::import_svg_bytes`], in the way of `mode`: objects,
    /// every shape as a running-stitch line (`SvgImport::as_lines`), or
    /// vectors with no stitches.
    pub fn import_svg_as(&mut self, bytes: &[u8], source: &str, mode: ArtworkImport) {
        // The file type from the name; SVG if there is none.
        let ext = std::path::Path::new(source)
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_else(|| "svg".into());
        let read = emb_formats::artwork::import_artwork(&ext, bytes, mode == ArtworkImport::Vectors);
        let imported = match read {
            Ok(i) if mode == ArtworkImport::Lines => i.as_lines(),
            Ok(i) => i,
            Err(e) => {
                self.notice = Some(Notice {
                    title: "Cannot import artwork".into(),
                    lines: vec![source.to_string(), e.to_string()],
                });
                return;
            }
        };
        if imported.objects.is_empty() {
            self.notice = Some(Notice {
                title: "Nothing to import".into(),
                lines: std::iter::once(tr("The file has no filled or stroked shapes.").to_string())
                    .chain(imported.warnings)
                    .collect(),
            });
            return;
        }
        let target = self.doc.design().hoop.center;
        let shift = imported.bbox().map(|b| target - b.center()).unwrap_or(Point::ZERO);
        let t = Transform::translate(shift.x, shift.y);
        // The threads, the start and end points, the machine functions and
        // the groups of the objects (Ink/Stitch files) come too.
        let ids: Vec<ObjectId> = self.doc.edit(tr("Import Artwork"), |d| imported.add_to(d, &t));
        let n = ids.len();
        self.selection.set_all(ids);
        self.fit_requested = true;
        self.message = crate::i18n::fill("Imported {} object(s)", &[&n]);

        let mut lines = imported.warnings;
        if let Some(b) = self.doc.design().bbox_of(self.selection.ids())
            && let HoopSelection::Preset { id } = &self.doc.design().hoop.selection
            && let Some(h) = self.hoops.get(id)
            && !h.fits(b.width(), b.height(), 0.0)
        {
            lines.push(crate::i18n::fill(
                "The artwork is {} × {} mm, larger than the hoop ({} × {} mm). \
                 Use the handles or Object Properties to resize it.",
                &[
                    &format!("{:.0}", b.width()),
                    &format!("{:.0}", b.height()),
                    &h.width_mm,
                    &h.height_mm,
                ],
            ));
        }
        if !lines.is_empty() {
            self.notice = Some(Notice {
                title: crate::i18n::fill("Imported {} object(s)", &[&n]),
                lines,
            });
        }
    }
}

/// The stitches of the machine file of a dialog: from the disk (with the
/// colors of a file next to it), or in the browser from the upload.
pub fn read_stitches(path: &Path) -> Result<emb_model::StitchDesign, String> {
    #[cfg(not(target_arch = "wasm32"))]
    return emb_library::read_machine_file(path);
    #[cfg(target_arch = "wasm32")]
    {
        let ext = machine_ext(path).unwrap_or_default();
        let bytes = crate::dialog::read(path).map_err(|e| e.to_string())?;
        emb_formats::read_machine(&ext, &bytes)
    }
}

/// An existing thread with this color, or a new one.
pub fn thread_for_color(d: &mut Design, color: Rgb) -> ThreadId {
    d.thread_for_color(color)
}

/// Shows the unsaved-changes prompt and the notice window.
pub fn dialogs(ctx: &egui::Context, st: &mut AppState) {
    if let Some(action) = st.confirm.clone() {
        let name = st
            .file_path
            .as_deref()
            .and_then(Path::file_name)
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| st.doc.design().name.clone());
        let mut choice = None;
        egui::Modal::new(egui::Id::new("unsaved_changes")).show(ctx, |ui| {
            ui.heading(tr("Save changes?"));
            ui.label(crate::i18n::fill("\"{}\" has changes that are not saved.", &[&name]));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button(tr("Save")).clicked() {
                    choice = Some(0);
                }
                if ui.button(tr("Don't save")).clicked() {
                    choice = Some(1);
                }
                if ui.button(tr("Cancel")).clicked() {
                    choice = Some(2);
                }
            });
        });
        match choice {
            Some(0) => {
                st.confirm = None;
                if st.save() {
                    st.perform(action);
                }
            }
            Some(1) => {
                st.confirm = None;
                st.perform(action);
            }
            Some(2) => {
                st.confirm = None;
                st.quit_queue.clear();
            }
            _ => {}
        }
    }

    if let Some(n) = st.notice.clone() {
        let mut close = false;
        egui::Modal::new(egui::Id::new("notice")).show(ctx, |ui| {
            ui.set_max_width(420.0);
            // The titles are English keys (ADR 0042); the lines have values in them.
            ui.heading(tr(&n.title));
            for l in &n.lines {
                ui.label(l);
            }
            ui.add_space(8.0);
            if ui.button(tr("OK")).clicked() {
                close = true;
            }
        });
        if close {
            st.notice = None;
        }
    }
}

/// The extension of a machine file that can be read, in lower case.
pub fn machine_ext(path: &Path) -> Option<String> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    emb_formats::READ_EXTENSIONS.contains(&ext.as_str()).then_some(ext)
}

#[cfg(test)]
mod tests {
    use emb_model::{HoopCatalog, MachineCatalog, ObjectKind};

    use super::*;

    fn state() -> AppState {
        AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new())
    }

    fn tmp_dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("emb-app-test-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn save_and_open_round_trip() {
        let dir = tmp_dir("save");
        let path = dir.join("patch.embproj");
        let mut st = state();
        st.add_sample_objects();
        assert!(st.doc.is_modified());
        assert!(st.save_to(&path));
        assert!(!st.doc.is_modified());
        assert_eq!(st.doc.design().name, "patch");
        assert!(st.window_title().starts_with("patch.embproj"));

        let saved = st.doc.design().clone();
        let mut other = state();
        other.open_path(&path);
        assert!(other.notice.is_none());
        assert!(other.doc.design().same_content(&saved));
        assert!(!other.doc.is_modified());
        assert_eq!(other.file_path.as_deref(), Some(path.as_path()));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn open_and_save_keep_a_list_of_recent_designs() {
        let dir = tmp_dir("recent");
        let mut st = state();
        st.add_sample_objects();
        let paths: Vec<PathBuf> = (0..MAX_RECENT + 2).map(|i| dir.join(format!("d{i}.embproj"))).collect();
        for p in &paths {
            assert!(st.save_to(p));
        }
        assert_eq!(st.settings.recent.len(), MAX_RECENT);
        assert_eq!(st.settings.recent[0], paths[MAX_RECENT + 1], "the last one first");
        // Opening a design again moves it to the top, with no copy.
        let mut other = state();
        other.settings = st.settings.clone();
        other.open_path(&paths[5]);
        assert_eq!(other.settings.recent[0], paths[5]);
        assert_eq!(other.settings.recent.iter().filter(|p| **p == paths[5]).count(), 1);
        // A file that is not there any more goes off the list.
        std::fs::remove_file(&paths[4]).unwrap();
        other.open_recent(&paths[4]);
        assert!(!other.settings.recent.contains(&paths[4]));
        assert!(other.confirm.is_none());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn open_any_opens_each_kind_of_file() {
        let dir = tmp_dir("any");
        let project = dir.join("patch.embproj");
        let mut st = state();
        st.add_sample_objects();
        assert!(st.save_to(&project));
        let svg = dir.join("shape.svg");
        std::fs::write(
            &svg,
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="40mm" height="20mm" viewBox="0 0 40 20"><rect width="40" height="20" fill="red"/></svg>"#,
        )
        .unwrap();
        let png = dir.join("PICTURE.PNG");
        image::RgbaImage::new(4, 2)
            .save_with_format(&png, image::ImageFormat::Png)
            .unwrap();

        let second = dir.join("second.embproj");
        std::fs::copy(&project, &second).unwrap();

        let mut other = state();
        other.open_any(&project);
        assert_eq!(other.tab_count(), 1, "the blank design gives its tab");
        assert_eq!(other.file_path.as_deref(), Some(project.as_path()));
        other.open_any(&second);
        assert_eq!(other.tab_count(), 2, "a project opens in a new tab");
        let objects = other.doc.design().objects().len();
        other.open_any(&svg);
        assert!(
            other.doc.design().objects().len() > objects,
            "artwork goes into the design"
        );
        other.open_any(&png);
        assert!(
            other.doc.design().backdrop.is_some(),
            "a picture (any case) becomes the backdrop"
        );
        other.open_any(&dir.join("missing.svg"));
        assert!(other.message.starts_with("Cannot read"), "{}", other.message);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn open_bad_file_shows_notice() {
        let dir = tmp_dir("bad");
        let path = dir.join("bad.embproj");
        std::fs::write(&path, "{ not json").unwrap();
        let mut st = state();
        st.open_path(&path);
        assert_eq!(st.notice.as_ref().unwrap().title, "Cannot open design");
        std::fs::remove_dir_all(dir).ok();
    }

    /// A change that puts an object out of the range of a project file is
    /// not made, a notice says so, and the design saves and opens again
    /// (ADR 0139).
    #[test]
    fn a_change_out_of_range_gives_a_notice_and_the_design_still_saves() {
        let dir = tmp_dir("far");
        let path = dir.join("far.embproj");
        let mut st = state();
        st.add_sample_objects();
        let ids: Vec<_> = st.doc.design().objects().iter().map(|o| o.id).collect();
        let before = project::to_string(st.doc.design());
        st.show_refused_change();
        assert!(st.notice.is_none());
        let far = emb_geom::Transform::translate(2.0 * emb_model::project::MAX_COORD_MM, 0.0);
        st.doc.edit("Move", |d| d.transform_objects(&ids, &far));
        st.show_refused_change();
        let n = st.notice.take().expect("a notice");
        assert_eq!(n.title, "The change was not made");
        assert!(n.lines[0].contains("out of range"), "{:?}", n.lines);
        assert_eq!(project::to_string(st.doc.design()), before);
        // The notice shows one time.
        st.show_refused_change();
        assert!(st.notice.is_none());
        assert!(st.save_to(&path));
        assert!(project::load(&path).is_ok());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn new_opens_a_tab_and_close_asks_first() {
        let mut st = state();
        st.request(Pending::New);
        assert!(st.confirm.is_none(), "empty design: no prompt");
        assert_eq!(st.tab_count(), 1, "the blank tab is used again");
        st.add_sample_objects();
        st.request(Pending::New);
        assert!(st.confirm.is_none(), "a new tab loses nothing");
        assert_eq!(st.tab_count(), 2);
        st.switch_tab(0);
        st.request(Pending::CloseTab);
        assert_eq!(st.confirm, Some(Pending::CloseTab));
        assert!(!st.doc.design().objects().is_empty(), "nothing discarded yet");
    }

    #[test]
    fn import_svg_centers_in_hoop_and_reuses_threads() {
        let mut st = state();
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="40mm" height="20mm" viewBox="0 0 40 20">
            <rect x="0" y="0" width="40" height="20" fill="#ff0000"/>
            <rect x="5" y="5" width="10" height="10" fill="#ff0000"/></svg>"##;
        let steps = st.doc.undo_len();
        st.import_svg_bytes(svg, "test.svg");
        assert!(st.notice.is_none(), "{:?}", st.notice);
        let d = st.doc.design();
        assert_eq!(d.objects().len(), 2);
        assert_eq!(d.threads.len(), 1, "same color, one thread");
        let b = d.bbox_of(st.selection.ids()).unwrap();
        assert!(b.center().distance(d.hoop.center) < 1e-6);
        assert_eq!(st.doc.undo_len(), steps + 1);
    }

    /// An Ink/Stitch file: a fill with a trim command and a satin column.
    const INKSTITCH: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"
        xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape" xmlns:inkstitch="http://inkstitch.org/namespace"
        width="60mm" height="40mm" viewBox="0 0 60 40">
        <metadata><inkstitch:inkstitch_svg_version>4</inkstitch:inkstitch_svg_version></metadata>
        <defs><symbol id="inkstitch_trim"><path d="M 0 0 L 2 2" style="stroke:#000000"/></symbol></defs>
        <rect id="leaf" x="0" y="0" width="30" height="20" style="fill:#00aa00" inkstitch:angle="45"/>
        <g><path id="c" d="M 5 5 L 0 0" style="fill:none;stroke:#000000" inkscape:connection-start="#u"
        inkscape:connection-end="#leaf"/><use id="u" xlink:href="#inkstitch_trim" x="5" y="5"/></g>
        <path id="stem" d="M 40 0 L 40 40 M 45 0 L 45 40" style="fill:none;stroke:#aa5500"
        inkstitch:satin_column="true" inkstitch:zigzag_spacing_mm="0.45"/></svg>"##;

    #[test]
    fn open_an_inkstitch_file_as_a_new_design_with_its_objects() {
        let dir = tmp_dir("inkstitch");
        let path = dir.join("leaf.svg");
        std::fs::write(&path, INKSTITCH).unwrap();
        let mut st = state();
        st.open_path(&path);
        assert!(st.notice.is_none(), "{:?}", st.notice);
        let d = st.doc.design();
        assert_eq!(d.name, "leaf");
        assert!(st.file_path.is_none());
        assert_eq!(d.objects().len(), 2);
        let ObjectKind::Fill { params, .. } = &d.objects()[0].kind else {
            panic!("{:?}", d.objects()[0].kind)
        };
        // Rows at -45° are rows at 135° (a move gives the angle from 0° to 180°).
        assert!((params.angle_deg.rem_euclid(180.0) - 135.0).abs() < 1e-6, "{params:?}");
        assert_eq!(d.objects()[0].functions.trim, emb_model::TrimMode::Always);
        assert!(matches!(d.objects()[1].kind, ObjectKind::Satin { .. }));
        assert_eq!(d.threads.len(), 2);
        let ids: Vec<_> = d.objects().iter().map(|o| o.id).collect();
        let b = d.bbox_of(&ids).unwrap();
        assert!(b.center().distance(d.hoop.center) < 1e-3);
        // A drop opens it too; plain artwork is imported into the design.
        let mut st = state();
        st.open_any(&path);
        assert_eq!(st.doc.design().name, "leaf");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn import_artwork_of_an_inkstitch_file_keeps_its_settings() {
        let mut st = state();
        st.import_svg_bytes(INKSTITCH, "leaf.svg");
        assert!(st.notice.is_none(), "{:?}", st.notice);
        let d = st.doc.design();
        let n = d.objects().len();
        assert!(matches!(d.objects()[n - 1].kind, ObjectKind::Satin { .. }));
        assert_eq!(d.objects()[n - 2].functions.trim, emb_model::TrimMode::Always);
    }

    #[test]
    fn import_svg_as_lines() {
        let mut st = state();
        // A frame (a fill with a hole) and a stroke.
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="40mm" height="20mm" viewBox="0 0 40 20">
            <path d="M0 0 H20 V20 H0 Z M5 5 V15 H15 V5 Z" fill="#00ff00" fill-rule="evenodd"/>
            <path d="M25 0 L40 20" stroke="#0000ff" fill="none"/></svg>"##;
        st.import_svg_as(svg, "test.svg", crate::vectors::ArtworkImport::Lines);
        let d = st.doc.design();
        assert_eq!(d.objects().len(), 2);
        assert!(
            d.objects()
                .iter()
                .all(|o| matches!(o.kind, emb_model::ObjectKind::Running { .. }))
        );
        let rings = d.objects()[0].kind.paths().next().unwrap().flatten(0.1);
        assert_eq!(rings.len(), 2, "the outline and the hole");
        assert!(rings.iter().all(|r| r.closed));
    }
}

#[cfg(test)]
mod machine_file_tests {
    use emb_model::{HoopCatalog, MachineCatalog, ObjectKind, StitchKind};

    use super::*;

    /// The sample design, written as a machine file in a new folder.
    fn sample_file(ext: &str) -> (AppState, std::path::PathBuf) {
        let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
        st.add_sample_objects();
        let plan = st.plan_view().plan.stitches.clone();
        let dir = std::env::temp_dir().join(format!("emb-open-{ext}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("sample.{ext}"));
        std::fs::write(
            &path,
            emb_formats::write_machine(ext, &plan, "sample", "20000101000000").unwrap(),
        )
        .unwrap();
        (st, path)
    }

    fn needles(st: &mut AppState) -> usize {
        st.plan_view().plan.stitches.stitch_count()
    }

    #[test]
    fn open_a_machine_file_as_stitch_objects() {
        for ext in ["dst", "vp3", "pes", "exp", "jef", "xxx"] {
            let (mut st, path) = sample_file(ext);
            let before = needles(&mut st);
            st.open_path(&path);
            assert!(st.notice.is_none(), "{ext}: {:?}", st.notice);
            let d = st.doc.design();
            assert!(
                d.objects()
                    .iter()
                    .all(|o| matches!(o.kind, ObjectKind::Stitches { .. }))
            );
            // Four color blocks in the sample: red, white, gold, red.
            assert_eq!(d.objects().len(), 4, "{ext}");
            assert_eq!(d.name, "sample");
            assert!(st.file_path.is_none());
            assert_eq!(needles(&mut st), before, "{ext}");
            std::fs::remove_dir_all(path.parent().unwrap()).ok();
        }
    }

    #[test]
    fn insert_recolor_split_and_break_apart() {
        let (mut st, path) = sample_file("vp3");
        let n = st.doc.design().objects().len();
        let threads = st.doc.design().threads.len();
        st.insert_machine_file(&path, "vp3");
        assert_eq!(st.doc.design().objects().len(), n + 4);
        assert_eq!(st.doc.design().threads.len(), threads, "same colors share threads");
        let first = st.selection.ids()[0];

        // Recolor: a new thread for the inserted object.
        st.selection.set(first);
        st.split_thread();
        assert_eq!(st.doc.design().threads.len(), threads + 1);

        // Split at the needle in the middle of the object.
        let span = st
            .plan_view()
            .plan
            .spans
            .iter()
            .find(|s| s.object == first)
            .unwrap()
            .clone();
        st.cursor = Some((span.records.start + span.records.end) / 2);
        let count = st.doc.design().objects().len();
        st.split_at_needle();
        assert_eq!(st.doc.design().objects().len(), count + 1, "{}", st.message);

        // Undo all: back to the sample.
        for _ in 0..3 {
            st.undo();
        }
        assert_eq!(st.doc.design().objects().len(), n);

        // Break apart the stitch object with the most parts (jumps or trims between them).
        st.open_path(&path);
        let parts = |o: &emb_model::DesignObject| match &o.kind {
            ObjectKind::Stitches { path, .. } => emb_stitch::stitch_subpaths(path).len(),
            _ => 0,
        };
        let obj = st
            .doc
            .design()
            .objects()
            .iter()
            .max_by_key(|o| parts(o))
            .unwrap()
            .clone();
        assert!(parts(&obj) > 1);
        let id = obj.id;
        st.selection.set(id);
        st.break_apart();
        assert!(st.doc.design().objects().len() > 4, "{}", st.message);
        let k = st
            .plan_view()
            .plan
            .stitches
            .records
            .iter()
            .filter(|r| r.kind == StitchKind::Stitch)
            .count();
        assert!(k > 0);
        std::fs::remove_dir_all(path.parent().unwrap()).ok();
    }

    #[test]
    fn bad_machine_files_give_a_notice() {
        let dir = std::env::temp_dir().join(format!("emb-bad-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bad.jef");
        std::fs::write(&path, [1, 2, 3]).unwrap();
        let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
        st.open_path(&path);
        assert!(st.notice.is_some());
        let empty = dir.join("empty.exp");
        std::fs::write(&empty, []).unwrap();
        st.notice = None;
        st.open_path(&empty);
        assert!(st.notice.is_some(), "no stitches");
        std::fs::remove_dir_all(dir).ok();
    }
}
