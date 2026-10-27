//! Output (roadmap P3, Milestone 5).
//!
//! - **Output design** makes all files for production in one step: the
//!   machine files, the project file, the production worksheet and the
//!   approval sheet (PDF), the design picture (PNG) and the design
//!   properties (CSV). The output profile (in the user settings) says which.
//! - **Print preview** shows the worksheet and the approval sheet as they
//!   print, and saves them as PDF.
//! - **Batch output** does the same for many project and machine files.

use std::path::{Path, PathBuf};

use eframe::egui::{self, Color32, FontId, Mesh, Rect, Sense, Stroke, Vec2};
use emb_model::{Design, FabricCatalog, HoopCatalog, HoopSelection, MachineCatalog, StitchDesign, project};
use emb_render::page::{Op, Page, Paper};
use emb_render::sheet::{self, SheetData};
use serde::{Deserialize, Serialize};

use crate::a11y::Named;
use crate::app::AppState;
use crate::canvas::color32;
use crate::file::Notice;
use crate::i18n::tr;

/// Which files Output design makes. A user setting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct OutputProfile {
    /// Machine formats by extension. Empty: the default format of the machine.
    pub formats: Vec<String>,
    pub worksheet: bool,
    pub approval: bool,
    pub picture: bool,
    /// The design picture as SVG (lines in mm, for vector programs).
    pub picture_svg: bool,
    pub csv: bool,
    /// Thread color files (INF, EDR, COL) next to the machine files, for
    /// the formats with no colors (DST and others).
    pub color_files: bool,
    /// The barcode on the production worksheet (the order number, else the
    /// file name).
    pub barcode: emb_render::barcode::BarcodeKind,
    /// The file names start with the order number of the design, if it has
    /// one (P13 Milestone 2).
    pub order_in_names: bool,
    pub paper: Paper,
    /// For a design with rhinestones: the template (SVG and PDF).
    pub stone_template: bool,
    /// The holes of a rhinestone template are this much larger than the
    /// stones, mm.
    pub stone_clearance_mm: f64,
    /// The PNG design picture has the flat thread look (else the 3D look
    /// with the default light, on the texture of the fabric).
    pub flat_picture: bool,
}

impl Default for OutputProfile {
    fn default() -> Self {
        OutputProfile {
            formats: Vec::new(),
            worksheet: true,
            approval: false,
            picture: true,
            picture_svg: false,
            csv: false,
            color_files: false,
            barcode: Default::default(),
            order_in_names: false,
            paper: Paper::Letter,
            stone_template: true,
            stone_clearance_mm: emb_model::bling::DEFAULT_CLEARANCE_MM,
            flat_picture: false,
        }
    }
}

/// Pixels per mm of the design picture.
const PICTURE_PX_PER_MM: f64 = 10.0;

/// The open Output design dialog.
#[derive(Debug, Clone)]
pub struct OutputDialog {
    pub folder: PathBuf,
    pub name: String,
}

/// The open print preview.
#[derive(Debug, Clone, Default)]
pub struct Preview {
    pub approval: bool,
    pub page: usize,
}

/// Today as `YYYY-MM-DD` (local time).
pub(crate) fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

/// Now as `YYYYMMDDhhmmss` (local time), for the JEF header.
pub fn local_stamp() -> String {
    chrono::Local::now().format("%Y%m%d%H%M%S").to_string()
}

/// The sheet data of a design and its stitches.
/// The preset lists that the sheets take names from.
#[derive(Clone, Copy)]
pub struct Catalogs<'a> {
    pub machines: &'a MachineCatalog,
    pub hoops: &'a HoopCatalog,
    pub fabrics: &'a FabricCatalog,
    pub sew_times: &'a std::collections::BTreeMap<String, emb_model::SewTimes>,
    /// The user's trims of DST and TAP files, by machine id.
    pub trims: &'a std::collections::BTreeMap<String, crate::setup::TrimSettings>,
    /// The needles of each multi-needle machine, by machine id (ADR 0063).
    pub needles: &'a std::collections::BTreeMap<String, emb_model::NeedleSetup>,
    /// The price lists and the currency, for the price on the approval
    /// sheet (ADR 0064).
    pub price_lists: &'a [emb_model::quote::PriceList],
    pub currency: &'a str,
    /// The thread stock and the default spool length, for the threads to
    /// buy on the worksheet (ADR 0066).
    pub stock: &'a emb_model::stock::ThreadStock,
    pub spool_m: f64,
}

impl Catalogs<'_> {
    /// The needle plan of `design` with the stitches `s` on its machine, or
    /// `None` for a machine with one needle (ADR 0063).
    pub fn needle_plan(&self, design: &Design, s: &StitchDesign) -> Option<emb_model::NeedlePlan> {
        let id = design.machine.as_deref()?;
        let m = self.machines.get(id)?;
        emb_model::needles::plan_for(design, s, m, self.needles.get(id))
    }

    /// The price list of the order of `design`: the list of its name, else
    /// the first list.
    pub fn price_list(&self, design: &Design) -> Option<&emb_model::quote::PriceList> {
        let name = design.order.as_ref().map(|o| o.price_list.as_str()).unwrap_or("");
        self.price_lists
            .iter()
            .find(|l| !name.is_empty() && l.name == name)
            .or(self.price_lists.first())
    }

    /// The price of the order of `design` as text, if it has an order and
    /// the price list has prices.
    pub fn order_price(&self, design: &Design, s: &StitchDesign) -> Option<String> {
        let order = design.order.as_ref()?;
        let list = self.price_list(design).filter(|l| l.is_set())?;
        let q = emb_model::quote::quote(list, emb_model::quote::Piece::of(s), &order.quote_order());
        Some(q.summary(self.currency))
    }
}

/// A copy of the preset lists, for a job that runs in the background.
#[derive(Clone)]
pub struct OwnedCatalogs {
    pub machines: MachineCatalog,
    pub hoops: HoopCatalog,
    pub fabrics: FabricCatalog,
    pub sew_times: std::collections::BTreeMap<String, emb_model::SewTimes>,
    pub trims: std::collections::BTreeMap<String, crate::setup::TrimSettings>,
    pub needles: std::collections::BTreeMap<String, emb_model::NeedleSetup>,
    pub price_lists: Vec<emb_model::quote::PriceList>,
    pub currency: String,
    pub stock: emb_model::stock::ThreadStock,
    pub spool_m: f64,
}

impl OwnedCatalogs {
    pub fn view(&self) -> Catalogs<'_> {
        Catalogs {
            machines: &self.machines,
            hoops: &self.hoops,
            fabrics: &self.fabrics,
            sew_times: &self.sew_times,
            trims: &self.trims,
            needles: &self.needles,
            price_lists: &self.price_lists,
            currency: &self.currency,
            stock: &self.stock,
            spool_m: self.spool_m,
        }
    }
}

/// Batch output: the files of the profile for each file in `paths`
/// (project or machine files) in `folder`. A machine file gets the machine
/// `machine`. Returns a line for each file.
pub fn batch_files(
    paths: &[PathBuf],
    folder: &Path,
    profile: &OutputProfile,
    c: Catalogs,
    machine: Option<&str>,
) -> Vec<String> {
    let mut lines = Vec::new();
    for p in paths {
        let name = p
            .file_stem()
            .map_or("design".into(), |s| s.to_string_lossy().into_owned());
        let design = match crate::file::machine_ext(p) {
            Some(_) => emb_library::read_machine_file(p).map(|s| {
                let mut d = emb_model::design_from_stitches(&s, &name);
                d.machine = machine.map(str::to_string);
                d
            }),
            None => project::load(p).map_err(|e| e.to_string()),
        };
        let result = design.and_then(|d| {
            let opts = d
                .machine
                .as_deref()
                .and_then(|m| c.machines.get(m))
                .map(emb_stitch::PlanOptions::for_machine)
                .unwrap_or_default();
            let plan = emb_stitch::build_plan(&d, &opts, &mut emb_stitch::StitchCache::default());
            write_files(&d, &plan, c, profile, folder, &name)
        });
        lines.push(match result {
            Ok(files) => format!("{}: {} files", p.display(), files.len()),
            Err(e) => format!("{}: not written: {e}", p.display()),
        });
    }
    lines
}

impl AppState {
    /// A copy of the preset lists for a background job.
    pub fn owned_catalogs(&self) -> OwnedCatalogs {
        OwnedCatalogs {
            machines: self.machines.clone(),
            hoops: self.hoops.clone(),
            fabrics: self.fabrics.clone(),
            sew_times: self.settings.sew_times.clone(),
            trims: self.settings.trims.clone(),
            needles: self.settings.needles.clone(),
            price_lists: self.settings.price_lists.clone(),
            currency: self.settings.thread_prices.currency.clone(),
            stock: self.settings.thread_stock.clone(),
            spool_m: self.settings.thread_prices.spool_m,
        }
    }

    pub fn catalogs(&self) -> Catalogs<'_> {
        Catalogs {
            machines: &self.machines,
            hoops: &self.hoops,
            fabrics: &self.fabrics,
            sew_times: &self.settings.sew_times,
            trims: &self.settings.trims,
            needles: &self.settings.needles,
            price_lists: &self.settings.price_lists,
            currency: &self.settings.thread_prices.currency,
            stock: &self.settings.thread_stock,
            spool_m: self.settings.thread_prices.spool_m,
        }
    }
}

pub fn sheet_data(design: &Design, stitches: &StitchDesign, c: Catalogs, files: Vec<String>) -> SheetData {
    let (machines, hoops, fabrics) = (c.machines, c.hoops, c.fabrics);
    let mut d = SheetData::from_stitches(&design.name, stitches);
    if let Some(plan) = c.needle_plan(design, stitches) {
        d.set_needles(&plan);
    }
    let machine = design.machine.as_deref().and_then(|m| machines.get(m));
    d.machine = machine.map(|m| m.name.clone());
    d.minutes = machine.map(|m| {
        c.sew_times
            .get(&m.id)
            .copied()
            .unwrap_or_else(|| m.sew_times())
            .minutes(d.stitches, d.colors.len().saturating_sub(1), d.trims, d.stops)
    });
    d.hoop = match &design.hoop.selection {
        HoopSelection::Preset { id } => Some(hoops.get(id).map_or(id.clone(), |h| h.name.clone())),
        HoopSelection::OpenField => None,
    };
    d.hoop_layout = match &design.hoop.selection {
        HoopSelection::Preset { id } => hoops
            .get(id)
            .and_then(|h| emb_render::sheet::HoopLayout::new(h, design, stitches)),
        HoopSelection::OpenField => None,
    };
    d.colorway = (design.colorways.len() > 1)
        .then(|| design.colorway_name().map(str::to_string))
        .flatten();
    if let Some(f) = design.fabric.as_deref().and_then(|id| fabrics.get(id)) {
        d.set_thread_use(stitches, f.thickness_mm);
        d.fabric = Some(f.name.clone());
        d.backing = Some(f.backing.clone());
        d.topping = f.topping.clone();
    }
    d.fabric_areas = sheet::fabric_areas(design);
    (d.stones, d.stone_counts) = sheet::stones_of(design);
    d.notes = design.notes.clone();
    d.order = design.order.clone();
    d.price = c.order_price(design, stitches);
    if !c.stock.is_empty() {
        let quantity = design.order.as_ref().map_or(1, |o| o.total_quantity());
        let thickness = fabrics.of(design).thickness_mm;
        d.to_buy = emb_model::stock::threads_to_buy(c.stock, stitches, thickness, quantity, c.spool_m)
            .iter()
            .map(|b| format!("{}: {} spools, {}", b.thread_text(), b.spools, b.reason.label()))
            .collect();
    }
    d.date = today();
    d.background = design.background;
    d.files = files;
    d
}

/// The machine formats to write: the profile's, or the machine default.
fn formats(profile: &OutputProfile, design: &Design, machines: &MachineCatalog) -> Vec<String> {
    let mut f: Vec<String> = profile
        .formats
        .iter()
        .filter(|e| emb_formats::WRITE_EXTENSIONS.contains(&e.as_str()))
        .cloned()
        .collect();
    if f.is_empty() {
        let default = design
            .machine
            .as_deref()
            .and_then(|m| machines.get(m))
            .map(|m| m.default_format.clone())
            .filter(|e| emb_formats::WRITE_EXTENSIONS.contains(&e.as_str()))
            .unwrap_or_else(|| "dst".into());
        f.push(default);
    }
    f
}

/// Writes the files of the profile for one design to `folder` as `name.*`.
/// The project file is written too. Returns the files.
pub fn write_files(
    design: &Design,
    plan: &emb_stitch::StitchPlan,
    c: Catalogs,
    profile: &OutputProfile,
    folder: &Path,
    name: &str,
) -> Result<Vec<PathBuf>, String> {
    std::fs::create_dir_all(folder).map_err(|e| format!("{}: {e}", folder.display()))?;
    // The order number before the name (an option of the profile).
    let with_order;
    let name = match design.order.as_ref().map(|o| o.file_part()) {
        Some(part) if profile.order_in_names && !part.is_empty() && !name.starts_with(&part) => {
            with_order = format!("{part} {name}");
            with_order.as_str()
        }
        _ => name,
    };
    let stitches = &plan.stitches;
    let mut written = Vec::new();
    let date = local_stamp();
    let write = |path: PathBuf, bytes: &[u8], written: &mut Vec<PathBuf>| -> Result<(), String> {
        std::fs::write(&path, bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        written.push(path);
        Ok(())
    };
    // A design of rhinestones only has no machine file.
    let sews = stitches.stitch_count() > 0;
    let machine = design
        .machine
        .as_deref()
        .and_then(|id| c.machines.get(id))
        .map(|m| crate::setup::machine_with(m, c.trims));
    let needles = c.needle_plan(design, stitches);
    for ext in formats(profile, design, c.machines).into_iter().filter(|_| sews) {
        let bytes = emb_formats::write_machine_with(&ext, stitches, name, &date, machine.as_ref(), needles.as_ref())
            .ok_or_else(|| format!(".{ext} cannot be written"))?;
        write(folder.join(format!("{name}.{ext}")), &bytes, &mut written)?;
    }
    if profile.color_files && sews {
        for ext in emb_formats::colors::EXTENSIONS {
            if let Some(bytes) = emb_formats::colors::write(ext, &stitches.threads) {
                write(folder.join(format!("{name}.{ext}")), &bytes, &mut written)?;
            }
        }
    }
    let proj = folder.join(format!("{name}.{}", project::FILE_EXTENSION));
    project::save(design, &proj).map_err(|e| e.to_string())?;
    written.push(proj);
    if profile.stone_template
        && let Some(t) = design.stone_template(profile.stone_clearance_mm)
    {
        write(
            folder.join(format!("{name} template.svg")),
            emb_formats::stones::svg(&t).as_bytes(),
            &mut written,
        )?;
        let spec = emb_render::stone_sheet::StoneSheetSpec {
            name: design.name.clone(),
            paper: profile.paper,
            clearance_mm: profile.stone_clearance_mm,
        };
        let pdf = emb_render::pdf::write(&emb_render::stone_sheet::pages(&t, &spec), &design.name);
        write(folder.join(format!("{name} template.pdf")), &pdf, &mut written)?;
    }

    let files: Vec<String> = written
        .iter()
        .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .collect();
    let mut data = sheet_data(design, stitches, c, files);
    data.stop_notes = plan.stop_notes(design);
    data.process_steps = sheet::process_steps(design, name);
    data.barcode =
        (profile.barcode != emb_render::barcode::BarcodeKind::None).then(|| (profile.barcode, data.barcode_text(name)));
    if profile.worksheet {
        let pdf = emb_render::pdf::write(&sheet::production(&data, profile.paper), &design.name);
        write(folder.join(format!("{name} worksheet.pdf")), &pdf, &mut written)?;
    }
    if profile.approval {
        let pdf = emb_render::pdf::write(&[sheet::approval(&data, profile.paper)], &design.name);
        write(folder.join(format!("{name} approval.pdf")), &pdf, &mut written)?;
    }
    if profile.picture {
        let look = crate::viewtools::LookSettings {
            three_d: !profile.flat_picture,
            ..Default::default()
        }
        .picture_look(design.fabric.as_deref());
        let pic = emb_render::raster::picture_look(
            &data.segments,
            &sheet::picture_areas(design),
            PICTURE_PX_PER_MM,
            0.4,
            2.0,
            design.background,
            &look,
        );
        write(folder.join(format!("{name}.png")), &pic.png(), &mut written)?;
    }
    if profile.picture_svg {
        let svg = emb_render::svg::picture(
            &data.segments,
            &sheet::picture_areas(design),
            0.4,
            2.0,
            design.background,
        );
        write(folder.join(format!("{name}.svg")), svg.as_bytes(), &mut written)?;
    }
    if profile.csv {
        let csv = format!("{}\n{}\n", sheet::csv_header(), sheet::csv_line(&data));
        write(folder.join(format!("{name}.csv")), csv.as_bytes(), &mut written)?;
    }
    Ok(written)
}

impl AppState {
    /// The sheet data of the current design.
    pub fn current_sheet_data(&mut self) -> SheetData {
        let plan = self.plan_view().plan.clone();
        let files = self
            .file_path
            .iter()
            .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .collect();
        let mut data = sheet_data(self.doc.design(), &plan.stitches, self.catalogs(), files);
        data.stop_notes = plan.stop_notes(self.doc.design());
        let kind = self.settings.output.barcode;
        let name = self
            .file_path
            .as_deref()
            .and_then(Path::file_stem)
            .map_or_else(|| self.doc.design().name.clone(), |s| s.to_string_lossy().into_owned());
        data.barcode = (kind != emb_render::barcode::BarcodeKind::None).then(|| (kind, data.barcode_text(&name)));
        data
    }

    /// Asks for a file name and writes the design picture as SVG.
    pub fn export_svg_dialog(&mut self) {
        let name = self
            .file_path
            .as_deref()
            .and_then(Path::file_stem)
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.doc.design().name.clone());
        let Some(path) = rfd::FileDialog::new()
            .set_title("Export design picture (SVG)")
            .add_filter("SVG picture", &["svg"])
            .set_directory(self.dialog_dir())
            .set_file_name(format!("{name}.svg"))
            .save_file()
        else {
            return;
        };
        let path = path.with_extension("svg");
        self.message = match std::fs::write(&path, self.design_svg()) {
            Ok(()) => format!("Exported {}", path.display()),
            Err(e) => format!("{}: {e}", path.display()),
        };
    }

    /// The design picture as SVG text.
    pub fn design_svg(&mut self) -> String {
        let data = self.current_sheet_data();
        emb_render::svg::picture(
            &data.segments,
            &sheet::picture_areas(self.doc.design()),
            0.4,
            2.0,
            self.doc.design().background,
        )
    }

    pub fn open_output_dialog(&mut self) {
        let folder = self.dialog_dir();
        let name = self
            .file_path
            .as_deref()
            .and_then(Path::file_stem)
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.doc.design().name.clone());
        self.output = Some(OutputDialog { folder, name });
    }

    /// Output design as a job (the Output button): the checks and the
    /// project file now, the other files in the background.
    pub fn queue_output_design(&mut self, folder: &Path, name: &str) -> Result<(), String> {
        let report = self.export_report();
        // A design of rhinestones only makes its template and sheets, with
        // no machine file.
        let stones_only = report.stitches == 0 && !self.placed_stones().is_empty();
        if let Some(e) = report
            .errors
            .iter()
            .find(|e| !(stones_only && e.text == crate::export::NO_STITCHES))
        {
            return Err(e.text.clone());
        }
        std::fs::create_dir_all(folder).map_err(|e| format!("{}: {e}", folder.display()))?;
        let proj = folder.join(format!("{name}.{}", project::FILE_EXTENSION));
        if !self.save_to(&proj) {
            return Err(format!("The project file {} was not saved.", proj.display()));
        }
        let design = self.doc.design().clone();
        let plan = self.plan_view().plan.clone();
        let profile = self.settings.output.clone();
        let catalogs = self.owned_catalogs();
        let (f, n) = (folder.to_path_buf(), name.to_string());
        let work: crate::jobs::Work = Box::new(move || {
            let files = write_files(&design, &plan, catalogs.view(), &profile, &f, &n)?;
            Ok(crate::jobs::JobOutput {
                message: format!("Output: {} files in {}", files.len(), f.display()),
                files,
                ..Default::default()
            })
        });
        let again = crate::jobs::Again::Batch {
            paths: vec![proj],
            folder: folder.to_path_buf(),
            profile: self.settings.output.clone(),
            machine: self.doc.design().machine.clone(),
        };
        self.queue_job("Output Design", name, Some(folder.to_path_buf()), Some(again), work);
        Ok(())
    }

    pub fn batch_output_dialog(&mut self) {
        let mut types = vec![project::FILE_EXTENSION];
        types.extend(emb_formats::READ_EXTENSIONS);
        let Some(paths) = rfd::FileDialog::new()
            .set_title("Batch output: designs")
            .add_filter("Designs and machine files", &types)
            .set_directory(self.dialog_dir())
            .pick_files()
        else {
            return;
        };
        let Some(folder) = rfd::FileDialog::new()
            .set_title("Batch output: folder for the files")
            .set_directory(self.dialog_dir())
            .pick_folder()
        else {
            return;
        };
        let (profile, machine) = (self.settings.output.clone(), self.doc.design().machine.clone());
        self.queue_batch(paths, folder, profile, machine);
        self.message = tr("Batch output is in the queue (the Exports tab shows it).").into();
    }

    /// The pages of the sheet that the preview shows.
    fn preview_pages(&mut self, approval: bool) -> Vec<Page> {
        let data = self.current_sheet_data();
        let paper = self.settings.output.paper;
        if approval {
            vec![sheet::approval(&data, paper)]
        } else {
            sheet::production(&data, paper)
        }
    }
}

/// Draws a page in `rect` (white paper with a shadow).
pub fn paint_page(painter: &egui::Painter, rect: Rect, page: &Page) {
    let s = (rect.width() / page.width_mm as f32).min(rect.height() / page.height_mm as f32);
    let size = Vec2::new(page.width_mm as f32 * s, page.height_mm as f32 * s);
    let origin = rect.center() - size / 2.0;
    let paper = Rect::from_min_size(origin, size);
    painter.rect_filled(paper.translate(Vec2::new(3.0, 3.0)), 0.0, Color32::from_black_alpha(60));
    painter.rect_filled(paper, 0.0, Color32::WHITE);
    let at = |p: emb_geom::Point| origin + Vec2::new(p.x as f32 * s, p.y as f32 * s);
    let painter = painter.with_clip_rect(paper);
    for op in &page.ops {
        match op {
            Op::Text {
                at: p,
                size_pt,
                bold,
                color,
                text,
            } => {
                let px = (*size_pt as f32 * 25.4 / 72.0 * s).max(1.0);
                let pos = at(*p);
                painter.text(
                    pos,
                    egui::Align2::LEFT_BOTTOM,
                    text,
                    FontId::proportional(px),
                    color32(*color),
                );
                if *bold {
                    painter.text(
                        pos + Vec2::new(px * 0.04, 0.0),
                        egui::Align2::LEFT_BOTTOM,
                        text,
                        FontId::proportional(px),
                        color32(*color),
                    );
                }
            }
            Op::Line { a, b, width_mm, color } => {
                painter.line_segment(
                    [at(*a), at(*b)],
                    Stroke::new((*width_mm as f32 * s).max(0.5), color32(*color)),
                );
            }
            Op::Rect { rect: r, fill, stroke } => {
                let rr = Rect::from_two_pos(at(r.min), at(r.max));
                if let Some(f) = fill {
                    painter.rect_filled(rr, 0.0, color32(*f));
                }
                if let Some((w, c)) = stroke {
                    painter.rect_stroke(
                        rr,
                        0.0,
                        Stroke::new((*w as f32 * s).max(0.5), color32(*c)),
                        egui::StrokeKind::Middle,
                    );
                }
            }
            Op::Area { rings, fill } => {
                let rings: Vec<Vec<egui::Pos2>> = rings.iter().map(|r| r.iter().map(|q| at(*q)).collect()).collect();
                painter.add(egui::Shape::mesh(crate::applique::scanline_mesh(
                    &rings,
                    color32(*fill),
                    painter.clip_rect(),
                )));
            }
            Op::Stitches { segments, width_mm } => {
                let half = (*width_mm as f32 * s / 2.0).max(0.4);
                let mut mesh = Mesh::default();
                for seg in segments {
                    let (a, b) = (at(seg.a), at(seg.b));
                    let d = b - a;
                    let len = d.length();
                    if len < 0.01 {
                        continue;
                    }
                    let n = Vec2::new(-d.y, d.x) / len * half;
                    let c = color32(seg.color);
                    let i = mesh.vertices.len() as u32;
                    for p in [a + n, a - n, b - n, b + n] {
                        mesh.colored_vertex(p, c);
                    }
                    mesh.add_triangle(i, i + 1, i + 2);
                    mesh.add_triangle(i, i + 2, i + 3);
                }
                painter.add(egui::Shape::mesh(mesh));
            }
        }
    }
}

/// The Output design dialog and the print preview window.
pub fn windows(ctx: &egui::Context, st: &mut AppState) {
    output_window(ctx, st);
    preview_window(ctx, st);
}

fn profile_ui(ui: &mut egui::Ui, st: &mut AppState) {
    let mut p = st.settings.output.clone();
    let default = formats(&OutputProfile::default(), st.doc.design(), &st.machines);
    ui.label(tr("Machine files"));
    ui.horizontal_wrapped(|ui| {
        for ext in emb_formats::WRITE_EXTENSIONS {
            let mut on = if p.formats.is_empty() {
                default.iter().any(|d| d == ext)
            } else {
                p.formats.iter().any(|f| f == ext)
            };
            if ui.checkbox(&mut on, ext.to_uppercase()).changed() {
                if p.formats.is_empty() {
                    p.formats = default.clone();
                }
                p.formats.retain(|f| f != ext);
                if on {
                    p.formats.push(ext.to_string());
                }
            }
        }
    });
    ui.add_enabled(false, egui::Checkbox::new(&mut true, tr("Project file (always)")));
    ui.checkbox(&mut p.worksheet, tr("Production worksheet (PDF)"));
    ui.checkbox(&mut p.approval, tr("Approval sheet (PDF)"));
    ui.horizontal(|ui| {
        ui.checkbox(&mut p.picture, tr("Design picture (PNG)"));
        ui.add_enabled_ui(p.picture, |ui| {
            let mut three_d = !p.flat_picture;
            if ui
                .checkbox(&mut three_d, tr("3D thread look"))
                .on_hover_text(tr(
                    "Light and shade on the threads and the fabric texture under the background color.",
                ))
                .changed()
            {
                p.flat_picture = !three_d;
            }
        });
    });
    ui.checkbox(&mut p.picture_svg, tr("Design picture (SVG)"))
        .on_hover_text(tr(
            "The stitches as lines in the thread colors, at real size, for vector programs.",
        ));
    ui.checkbox(&mut p.csv, tr("Design properties (CSV)"));
    ui.horizontal(|ui| {
        ui.label(tr("Barcode on the worksheet"));
        egui::ComboBox::from_id_salt("output-barcode")
            .selected_text(p.barcode.label())
            .show_ui(ui, |ui| {
                for k in emb_render::barcode::BarcodeKind::ALL {
                    ui.selectable_value(&mut p.barcode, k, k.label());
                }
            });
    })
    .response
    .on_hover_text(tr("The order number (else the file name) as a barcode, so a shop system or a phone can find the order. Code 39 holds capital letters, digits and - . $ / + % only."));
    ui.checkbox(&mut p.order_in_names, tr("Order number before the file names"))
        .on_hover_text(tr(
            "For a design with an order number (Design > Order Details), for example \"A-1042 rose.dst\".",
        ));
    ui.checkbox(&mut p.color_files, tr("Thread color files (INF, EDR, COL)"))
        .on_hover_text(tr(
            "The thread colors next to the machine files, for DST and the other formats with no colors.",
        ));
    ui.horizontal(|ui| {
        ui.checkbox(&mut p.stone_template, tr("Rhinestone template (SVG, PDF)"))
            .on_hover_text(tr(
                "For a design with rhinestones: the template for the cutter, and the template on paper.",
            ));
        crate::bling::clearance_ui(ui, &mut p.stone_clearance_mm);
    });
    ui.horizontal(|ui| {
        ui.label(tr("Paper"));
        for paper in [Paper::Letter, Paper::A4] {
            ui.selectable_value(&mut p.paper, paper, paper.label());
        }
    });
    st.set_settings(crate::workspace::Settings {
        output: p,
        ..st.settings.clone()
    });
}

fn output_window(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut dlg) = st.output.clone() else { return };
    let mut open = true;
    let mut run = false;
    egui::Window::new(tr("Output Design"))
        .id(egui::Id::new("Output Design"))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            egui::Grid::new("output_where").num_columns(2).show(ui, |ui| {
                ui.label(tr("Folder"));
                ui.horizontal(|ui| {
                    ui.label(dlg.folder.display().to_string());
                    if ui.button(tr("Change…")).clicked()
                        && let Some(f) = rfd::FileDialog::new().set_directory(&dlg.folder).pick_folder()
                    {
                        dlg.folder = f;
                    }
                });
                ui.end_row();
                ui.label(tr("Name"));
                ui.text_edit_singleline(&mut dlg.name);
                ui.end_row();
            });
            ui.separator();
            profile_ui(ui, st);
            ui.separator();
            let report = st.export_report();
            for e in &report.errors {
                ui.colored_label(Color32::from_rgb(220, 60, 50), format!("⛔ {}", e.text));
            }
            ui.horizontal(|ui| {
                if ui.button(tr("Print Preview…")).clicked() {
                    st.preview = Some(Preview::default());
                }
                let ok = report.errors.is_empty() && !dlg.name.trim().is_empty();
                if ui.add_enabled(ok, egui::Button::new(tr("Output"))).clicked() {
                    run = true;
                }
            });
        });
    if run {
        let name = dlg.name.trim().to_string();
        match st.queue_output_design(&dlg.folder, &name) {
            Ok(()) => {
                st.message = crate::i18n::fill("Output of {} is in the queue (the Exports tab shows it).", &[&name]);
                st.output = None;
            }
            Err(e) => {
                st.notice = Some(Notice {
                    title: "Output not written".into(),
                    lines: vec![e],
                });
            }
        }
        return;
    }
    st.output = open.then_some(dlg);
}

fn preview_window(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut pv) = st.preview.clone() else { return };
    let pages = st.preview_pages(pv.approval);
    pv.page = pv.page.min(pages.len().saturating_sub(1));
    let mut open = true;
    egui::Window::new(tr("Print Preview"))
        .id(egui::Id::new("Print Preview"))
        .open(&mut open)
        .default_size([620.0, 760.0])
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut pv.approval, false, tr("Production worksheet"));
                ui.selectable_value(&mut pv.approval, true, tr("Approval sheet"));
                ui.separator();
                if ui
                    .add_enabled(pv.page > 0, egui::Button::new("◀"))
                    .named(tr("Previous page"))
                    .clicked()
                {
                    pv.page -= 1;
                }
                ui.label(crate::i18n::fill("Page {} of {}", &[&(pv.page + 1), &(pages.len())]));
                if ui
                    .add_enabled(pv.page + 1 < pages.len(), egui::Button::new("▶"))
                    .named(tr("Next page"))
                    .clicked()
                {
                    pv.page += 1;
                }
                ui.separator();
                if ui.button(tr("Save PDF…")).clicked() {
                    let kind = if pv.approval { "approval" } else { "worksheet" };
                    let name = format!("{} {kind}.pdf", st.doc.design().name);
                    if let Some(path) = rfd::FileDialog::new()
                        .set_file_name(name)
                        .add_filter("PDF", &["pdf"])
                        .set_directory(st.dialog_dir())
                        .save_file()
                    {
                        let pdf = emb_render::pdf::write(&pages, &st.doc.design().name);
                        st.message = match std::fs::write(&path, pdf) {
                            Ok(()) => format!("Saved {}", path.display()),
                            Err(e) => format!("Not saved: {e}"),
                        };
                    }
                }
            });
            ui.weak(tr("To print, save the PDF and print it from a PDF viewer."));
            let (rect, _) = ui.allocate_exact_size(ui.available_size().max(Vec2::new(300.0, 380.0)), Sense::hover());
            if let Some(page) = pages.get(pv.page) {
                paint_page(ui.painter(), rect.shrink(8.0), page);
            }
        });
    st.preview = open.then_some(pv);
}

/// Notes for the worksheets, in the Design info panel.
pub fn notes_ui(ui: &mut egui::Ui, st: &mut AppState) {
    ui.strong(tr("Notes for the worksheets"));
    let mut notes = st.doc.design().notes.clone();
    let r = ui.add(
        egui::TextEdit::multiline(&mut notes)
            .desired_rows(3)
            .desired_width(f32::INFINITY)
            .hint_text(tr("Job, customer, fabric, stabilizer, needle…")),
    );
    if r.changed() {
        st.doc
            .edit_merged("Notes", emb_model::MergeKey::new("design-notes"), |d| d.notes = notes);
    }
    if r.lost_focus() {
        st.doc.end_merge();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> AppState {
        let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
        st.add_sample_objects();
        st.doc.edit("Notes", |d| d.notes = "Tear-away stabilizer.".into());
        st
    }

    /// Output design through the queue; the files of the job.
    fn output(st: &mut AppState, dir: &Path, name: &str) -> Result<Vec<PathBuf>, String> {
        st.queue_output_design(dir, name)?;
        st.wait_jobs();
        let j = st.jobs.jobs.last().unwrap();
        match &j.state {
            crate::jobs::JobState::Done => Ok(j.files.clone()),
            other => Err(format!("{other:?}")),
        }
    }

    /// A design of rhinestones only: Output design makes the template and
    /// the sheets, and no machine file; the picture shows the stones.
    #[test]
    fn rhinestones_only_output_has_no_machine_file() {
        let mut st = state();
        st.new_design();
        st.doc.edit("t", |d| {
            let t = d.add_thread("Stones", emb_model::Rgb(0, 0, 0));
            d.add_object(
                t,
                emb_model::ObjectKind::Bling(Box::new(emb_model::Bling::new(
                    Default::default(),
                    emb_model::BlingLayout::Stones {
                        points: vec![emb_geom::Point::ZERO, emb_geom::Point::new(10.0, 0.0)],
                    },
                ))),
            );
        });
        let dir = temp("stones-only");
        let files = output(&mut st, &dir, "stones").unwrap();
        let names: Vec<String> = files
            .iter()
            .filter_map(|f| f.file_name().map(|n| n.to_string_lossy().into_owned()))
            .collect();
        assert!(names.contains(&"stones template.svg".to_string()), "{names:?}");
        assert!(names.contains(&"stones worksheet.pdf".to_string()));
        assert!(
            !names.iter().any(|n| n.ends_with(".vp3") || n.ends_with(".dst")),
            "{names:?}"
        );
        // The picture is the size of the stones (about 13 x 3 mm and the margins).
        let png = image::open(dir.join("stones.png")).unwrap();
        assert!(
            png.width() > 100 && png.height() > 20,
            "{}x{}",
            png.width(),
            png.height()
        );
        // Export of a machine file still needs stitches.
        assert!(
            st.export_report()
                .errors
                .iter()
                .any(|e| e.text == crate::export::NO_STITCHES)
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    fn temp(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("emb-output-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn one_command_makes_all_files() {
        let mut st = state();
        st.settings.output = OutputProfile {
            formats: vec!["vp3".into(), "dst".into()],
            worksheet: true,
            approval: true,
            picture: true,
            picture_svg: true,
            csv: true,
            color_files: true,
            barcode: emb_render::barcode::BarcodeKind::Code39,
            paper: Paper::A4,
            ..OutputProfile::default()
        };
        let dir = temp("all");
        let files = output(&mut st, &dir, "patch").unwrap();
        let names: Vec<String> = files
            .iter()
            .map(|f| f.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            names,
            [
                "patch.vp3",
                "patch.dst",
                "patch.inf",
                "patch.edr",
                "patch.col",
                "patch.embproj",
                "patch worksheet.pdf",
                "patch approval.pdf",
                "patch.png",
                "patch.svg",
                "patch.csv"
            ]
        );
        for f in &files {
            assert!(std::fs::metadata(f).unwrap().len() > 0, "{}", f.display());
        }
        let pdf = std::fs::read(dir.join("patch worksheet.pdf")).unwrap();
        assert!(pdf.starts_with(b"%PDF-1.4"));
        let back = emb_formats::vp3::read(&std::fs::read(dir.join("patch.vp3")).unwrap()).unwrap();
        assert_eq!(back.stitch_count(), st.plan_view().plan.stitches.stitch_count());
        assert_eq!(st.file_path.as_deref(), Some(dir.join("patch.embproj").as_path()));
        assert!(!st.doc.is_modified());
        // The DST file reads with the colors of the color files next to it.
        let dst = emb_library::read_machine_file(&dir.join("patch.dst")).unwrap();
        let plan_threads = st.plan_view().plan.stitches.threads.clone();
        assert_eq!(
            dst.threads.iter().map(|t| t.color).collect::<Vec<_>>(),
            plan_threads.iter().map(|t| t.color).collect::<Vec<_>>()
        );
        let csv = std::fs::read_to_string(dir.join("patch.csv")).unwrap();
        assert_eq!(csv.lines().count(), 2);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn default_profile_uses_the_machine_format() {
        let mut st = state();
        let dir = temp("default");
        let files = output(&mut st, &dir, "p").unwrap();
        let exts: Vec<String> = files
            .iter()
            .map(|f| f.extension().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(exts, ["vp3", "embproj", "pdf", "png"], "the Topaz 20 reads VP3");
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn errors_block_the_output() {
        let mut st = state();
        st.doc.edit("Far", |d| {
            let t = d.threads[0].id;
            d.add_object(
                t,
                emb_model::ObjectKind::Running {
                    path: emb_geom::Path::new()
                        .move_to(emb_geom::Point::new(300.0, 0.0))
                        .line_to(emb_geom::Point::new(320.0, 0.0)),
                    params: Default::default(),
                },
            );
        });
        let dir = temp("error");
        assert!(output(&mut st, &dir, "p").is_err());
        assert!(st.jobs.jobs.is_empty(), "nothing is queued");
        assert!(!dir.join("p.vp3").exists());
    }

    #[test]
    fn batch_output_of_projects_and_machine_files() {
        let mut st = state();
        let dir = temp("batch-in");
        std::fs::create_dir_all(&dir).unwrap();
        st.save_to(&dir.join("a.embproj"));
        let plan = st.plan_view().plan.stitches.clone();
        std::fs::write(dir.join("b.dst"), emb_formats::dst::write(&plan, "b")).unwrap();
        std::fs::write(dir.join("c.jef"), [1, 2, 3]).unwrap();
        let out = temp("batch-out");
        let machine = st.doc.design().machine.clone();
        let lines = batch_files(
            &[dir.join("a.embproj"), dir.join("b.dst"), dir.join("c.jef")],
            &out,
            &st.settings.output,
            st.catalogs(),
            machine.as_deref(),
        );
        assert!(lines[0].ends_with("4 files"), "{lines:?}");
        assert!(lines[1].ends_with("4 files"), "{lines:?}");
        assert!(lines[2].contains("not written"), "{lines:?}");
        assert!(out.join("b.vp3").exists() && out.join("b worksheet.pdf").exists());
        std::fs::remove_dir_all(dir).ok();
        std::fs::remove_dir_all(out).ok();
    }

    #[test]
    fn preview_and_dialog_draw() {
        let mut st = state();
        st.open_output_dialog();
        st.preview = Some(Preview::default());
        let ctx = egui::Context::default();
        for approval in [false, true] {
            st.preview = Some(Preview { approval, page: 0 });
            let mut out = ctx.run_ui(Default::default(), |ui| windows(&ui.ctx().clone(), &mut st));
            out.textures_delta.clear();
        }
        assert!(st.output.is_some() && st.preview.is_some());
    }
}
