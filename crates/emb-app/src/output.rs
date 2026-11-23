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
use crate::dock::FitScreen;
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
    /// The approval sheet has a page with the design on the product, if
    /// the design has a product (P16 Milestone 4).
    pub approval_product: bool,
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
    /// The dots for each inch of the PNG design picture (ADR 0130).
    pub picture_dpi: u32,
    /// The PNG design picture has no background (for a web shop).
    pub clear_picture: bool,
    /// The sew-out animation (GIF): the design as it sews.
    pub sewout_gif: bool,
}

impl Default for OutputProfile {
    fn default() -> Self {
        OutputProfile {
            formats: Vec::new(),
            worksheet: true,
            approval: false,
            approval_product: true,
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
            picture_dpi: PICTURE_DPI[1],
            clear_picture: false,
            sewout_gif: false,
        }
    }
}

/// The dots for each inch of the design picture that the Output window
/// offers (the second is the default: 10 pixels for each mm).
pub const PICTURE_DPI: [u32; 4] = [150, 254, 300, 600];

/// The look of the pictures of the approval sheet and of the PNG design
/// picture: the 3D look in the default light, on the fabric of the design.
fn sheet_look(design: &Design, fabrics: &FabricCatalog, flat: bool) -> emb_render::raster::Look {
    crate::viewtools::LookSettings {
        three_d: !flat,
        ..Default::default()
    }
    .picture_look(Some(emb_render::fabric::FabricLook::of(&fabrics.of(design), design)))
}

/// The records of each object of `plan`.
fn spans_of(plan: &emb_stitch::StitchPlan) -> impl Iterator<Item = (emb_model::ObjectId, std::ops::Range<usize>)> + '_ {
    plan.spans.iter().map(|s| (s.object, s.records.clone()))
}
/// Pixels per mm of the picture of the design on the product, on the
/// approval sheet (about 250 dpi on the page for one side).
const SHEET_PX_PER_MM: f64 = 2.5;

/// The page of the approval sheet with the design on its product, if the
/// design has a product.
fn product_page(
    design: &Design,
    plan: &emb_stitch::StitchPlan,
    machines: &MachineCatalog,
    data: &SheetData,
    paper: Paper,
) -> Option<Page> {
    let v = design.product.as_ref()?;
    let pictures = sheet_pictures(design, plan, machines, v);
    (!pictures.is_empty()).then(|| sheet::product_page(data, &pictures, paper))
}

/// The pictures of the product for the approval sheet, each with its caption.
/// The picture of the design on the approval sheet and the area that it
/// shows (`SheetData::picture`).
pub type SheetPicture = Option<(std::sync::Arc<emb_render::raster::Picture>, emb_geom::BBox)>;
pub type SheetPictures = Vec<(std::sync::Arc<emb_render::raster::Picture>, String)>;

/// The pictures of the design on its product `v` for the approval sheet,
/// each with its caption (the product and the side).
fn sheet_pictures(
    design: &Design,
    plan: &emb_stitch::StitchPlan,
    machines: &MachineCatalog,
    v: &emb_model::ProductView,
) -> SheetPictures {
    crate::product::pictures_of_design(design, plan, machines, &Default::default(), SHEET_PX_PER_MM)
        .into_iter()
        .map(|(side, pic)| {
            let caption = if v.picture.is_some() {
                tr("My picture").to_string()
            } else {
                tr(v.kind.on(side).name()).to_string()
            };
            (std::sync::Arc::new(pic), caption)
        })
        .collect()
}

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
/// `machine`. Returns a line for each file (`Err` if its files are not
/// written).
pub fn batch_files(
    paths: &[PathBuf],
    folder: &Path,
    profile: &OutputProfile,
    c: Catalogs,
    machine: Option<&str>,
) -> Vec<Result<String, String>> {
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
            Ok(files) => Ok(crate::i18n::fill("{}: {} files", &[&p.display(), &files.len()])),
            Err(e) => Err(crate::i18n::fill("{}: not written: {}", &[&p.display(), &e])),
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
        d.topping = f.topping_for(design).map(str::to_string);
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
            .map(|b| {
                crate::i18n::fill(
                    "{}: {} spools, {}",
                    &[&b.thread_text(), &b.spools, &tr(b.reason.label())],
                )
            })
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
            .ok_or_else(|| crate::i18n::fill(".{} cannot be written", &[&ext]))?;
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
            name: crate::file::shown_name(&design.name),
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
    let ground = Some(emb_render::fabric::Ground::of(&c.fabrics.of(design), design));
    if profile.approval {
        // The picture of the design with the surface of the 3D look (ADR 0130).
        let e = emb_render::picture::Embroidery::of_design(design, stitches, spans_of(plan), ground.clone());
        data.set_picture(&e, &sheet_look(design, c.fabrics, false));
        let mut pages = vec![sheet::approval(&data, profile.paper)];
        if profile.approval_product
            && let Some(page) = product_page(design, plan, c.machines, &data, profile.paper)
        {
            pages.push(page);
        }
        let pdf = emb_render::pdf::write(&pages, &design.name);
        write(folder.join(format!("{name} approval.pdf")), &pdf, &mut written)?;
    }
    if profile.picture {
        let pic = emb_render::picture::design_picture(
            design,
            stitches,
            spans_of(plan),
            ground.clone(),
            f64::from(profile.picture_dpi.clamp(72, 600)) / 25.4,
            2.0,
            design.background.filter(|_| !profile.clear_picture),
            &sheet_look(design, c.fabrics, profile.flat_picture),
        );
        write(folder.join(format!("{name}.png")), &pic.png(), &mut written)?;
    }
    if profile.sewout_gif && sews {
        let gif = emb_render::picture::sewout_gif(
            design,
            stitches,
            spans_of(plan),
            ground,
            &sheet_look(design, c.fabrics, false),
        )?;
        write(folder.join(format!("{name} sew-out.gif")), &gif, &mut written)?;
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
        let Some(path) = crate::dialog::FileDialog::new()
            .set_title(tr("Export design picture (SVG)"))
            .add_filter(tr("SVG picture"), &["svg"])
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

    /// Asks for a file name and writes the design as an SVG file of
    /// Ink/Stitch (ADR 0136).
    pub fn export_inkstitch_dialog(&mut self) {
        let name = self
            .file_path
            .as_deref()
            .and_then(Path::file_stem)
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.doc.design().name.clone());
        let Some(path) = crate::dialog::FileDialog::new()
            .set_title(tr("Export for Ink/Stitch"))
            .add_filter(tr("Ink/Stitch design (SVG)"), &["svg"])
            .set_directory(self.dialog_dir())
            .set_file_name(format!("{name}.svg"))
            .save_file()
        else {
            return;
        };
        self.export_inkstitch_to(&path.with_extension("svg"));
    }

    /// Writes the design as an SVG file of Ink/Stitch at `path`; the things
    /// that the file does not have show in a notice.
    pub fn export_inkstitch_to(&mut self, path: &Path) {
        let written =
            emb_formats::inkstitch::write::write_inkstitch(self.doc.design(), &emb_stitch::object::underlay_layers);
        match std::fs::write(path, written.svg) {
            Ok(()) => {
                self.message = crate::i18n::fill("Written for Ink/Stitch: {}", &[&(path.display())]);
                if !written.warnings.is_empty() {
                    self.notice = Some(crate::file::Notice {
                        title: "Written with notes".into(),
                        lines: written.warnings,
                    });
                }
            }
            Err(e) => {
                self.notice = Some(crate::file::Notice {
                    title: "Cannot write file".into(),
                    lines: vec![path.display().to_string(), e.to_string()],
                })
            }
        }
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
            return Err(crate::i18n::fill(
                "The project file {} was not saved.",
                &[&proj.display()],
            ));
        }
        let design = self.doc.design().clone();
        let plan = self.plan_view().plan.clone();
        let profile = self.settings.output.clone();
        let catalogs = self.owned_catalogs();
        let (f, n) = (folder.to_path_buf(), name.to_string());
        let work: crate::jobs::Work = Box::new(move || {
            let files = write_files(&design, &plan, catalogs.view(), &profile, &f, &n)?;
            Ok(crate::jobs::JobOutput {
                message: crate::i18n::fill("Output: {} files in {}", &[&files.len(), &f.display()]),
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
        let Some(paths) = crate::dialog::FileDialog::new()
            .set_title(tr("Batch output: designs"))
            .add_filter(tr("Designs and machine files"), &types)
            .set_directory(self.dialog_dir())
            .pick_files()
        else {
            return;
        };
        let Some(folder) = crate::dialog::FileDialog::new()
            .set_title(tr("Batch output: folder for the files"))
            .set_directory(self.dialog_dir())
            .pick_folder()
        else {
            return;
        };
        let (profile, machine) = (self.settings.output.clone(), self.doc.design().machine.clone());
        self.queue_batch(paths, folder, profile, machine);
        self.message = tr("Batch output is in the queue (the Exports tab shows it).").into();
    }

    /// The picture of the approval sheet and the quote of the design now
    /// (ADR 0130), kept for the state of the design.
    pub fn sheet_picture(&mut self, data: &mut SheetData) {
        let state = self.doc.state_id();
        if self.sheet_picture.as_ref().is_none_or(|(s, _)| *s != state) {
            let plan = self.plan_view().plan.clone();
            let d = self.doc.design();
            let ground = Some(emb_render::fabric::Ground::of(&self.fabrics.of(d), d));
            let e = emb_render::picture::Embroidery::of_design(d, &plan.stitches, spans_of(&plan), ground);
            data.set_picture(&e, &sheet_look(d, &self.fabrics, false));
            self.sheet_picture = Some((state, data.picture.clone()));
        }
        if let Some((_, p)) = &self.sheet_picture {
            data.picture = p.clone();
        }
    }

    /// The pages of the sheet that the preview shows.
    fn preview_pages(&mut self, approval: bool) -> Vec<Page> {
        let mut data = self.current_sheet_data();
        let paper = self.settings.output.paper;
        if approval {
            self.sheet_picture(&mut data);
            let mut pages = vec![sheet::approval(&data, paper)];
            if self.settings.output.approval_product
                && let Some(v) = self.doc.design().product.clone()
            {
                let state = self.doc.state_id();
                if self.sheet_product.as_ref().is_none_or(|(s, _)| *s != state) {
                    let plan = self.plan_view().plan.clone();
                    let pictures = sheet_pictures(self.doc.design(), &plan, &self.machines, &v);
                    self.sheet_product = Some((state, pictures));
                }
                if let Some((_, pictures)) = &self.sheet_product
                    && !pictures.is_empty()
                {
                    pages.push(sheet::product_page(&data, pictures, paper));
                }
            }
            pages
        } else {
            sheet::production(&data, paper)
        }
    }
}

/// The texture of the picture `k` of a page. The texture stays while the
/// picture is the same (the same `Arc`).
fn page_texture(
    ctx: &egui::Context,
    k: usize,
    picture: &std::sync::Arc<emb_render::raster::Picture>,
) -> egui::TextureHandle {
    let id = egui::Id::new(("page-picture", k));
    let ptr = std::sync::Arc::as_ptr(picture) as usize;
    if let Some((p, t)) = ctx.data(|d| d.get_temp::<(usize, egui::TextureHandle)>(id))
        && p == ptr
    {
        return t;
    }
    let image =
        egui::ColorImage::from_rgba_unmultiplied([picture.width as usize, picture.height as usize], &picture.rgba);
    let t = crate::resources::load_texture(ctx, format!("page-picture-{k}"), image, egui::TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(id, (ptr, t.clone())));
    t
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
    for (k, op) in page.ops.iter().enumerate() {
        match op {
            Op::Image { rect: r, picture } => {
                let tex = page_texture(painter.ctx(), k, picture);
                painter.image(
                    tex.id(),
                    Rect::from_two_pos(at(r.min), at(r.max)),
                    Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
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
    ui.horizontal(|ui| {
        ui.checkbox(&mut p.approval, tr("Approval sheet (PDF)"));
        ui.add_enabled(
            p.approval,
            egui::Checkbox::new(&mut p.approval_product, tr("With the product")),
        )
        .on_hover_text(tr(
            "A second page shows the design on its product (Design > Product), if the design has a product.",
        ));
    });
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
    ui.add_enabled_ui(p.picture, |ui| {
        ui.horizontal(|ui| {
            ui.add_space(24.0);
            egui::ComboBox::from_id_salt("output-dpi")
                .selected_text(crate::i18n::fill("{} dpi", &[&p.picture_dpi]))
                .show_ui(ui, |ui| {
                    for dpi in PICTURE_DPI {
                        ui.selectable_value(&mut p.picture_dpi, dpi, crate::i18n::fill("{} dpi", &[&dpi]));
                    }
                })
                .response
                .on_hover_text(tr(
                    "The dots for each inch of the picture at the real size of the design.",
                ));
            ui.checkbox(&mut p.clear_picture, tr("No background")).on_hover_text(tr(
                "A clear background round the stitches (for a web shop), not the fabric color.",
            ));
        });
    });
    ui.checkbox(&mut p.sewout_gif, tr("Sew-out animation (GIF)"))
        .on_hover_text(tr("An animated picture of the design as it sews, color by color."));
    ui.checkbox(&mut p.picture_svg, tr("Design picture (SVG)"))
        .on_hover_text(tr(
            "The stitches as lines in the thread colors, at real size, for vector programs.",
        ));
    ui.checkbox(&mut p.csv, tr("Design properties (CSV)"));
    ui.horizontal(|ui| {
        ui.label(tr("Barcode on the worksheet"));
        egui::ComboBox::from_id_salt("output-barcode")
            .selected_text(tr(p.barcode.label()))
            .show_ui(ui, |ui| {
                for k in emb_render::barcode::BarcodeKind::ALL {
                    ui.selectable_value(&mut p.barcode, k, tr(k.label()));
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
        for paper in Paper::ALL {
            ui.selectable_value(&mut p.paper, paper, tr(paper.label()));
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
        .fit_screen(ctx)
        .show(ctx, |ui| {
            egui::Grid::new("output_where").num_columns(2).show(ui, |ui| {
                ui.label(tr("Folder"));
                ui.horizontal(|ui| {
                    ui.label(dlg.folder.display().to_string());
                    if ui.button(tr("Change…")).clicked()
                        && let Some(f) = crate::dialog::FileDialog::new()
                            .set_directory(&dlg.folder)
                            .pick_folder()
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
        .fit_screen(ctx)
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
                    if let Some(path) = crate::dialog::FileDialog::new()
                        .set_file_name(name)
                        .add_filter("PDF", &["pdf"])
                        .set_directory(st.dialog_dir())
                        .save_file()
                    {
                        let pdf = emb_render::pdf::write(&pages, &st.doc.design().name);
                        st.message = match std::fs::write(&path, pdf) {
                            Ok(()) => crate::i18n::fill("Saved {}", &[&path.display()]),
                            Err(e) => crate::i18n::fill("Not saved: {}", &[&e]),
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
            .edit_merged(tr("Notes"), emb_model::MergeKey::new("design-notes"), |d| {
                d.notes = notes
            });
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
        // The test is on the reference machine.
        st.start_on_machine("husqvarna_topaz20", None);
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
    fn the_approval_sheet_has_the_product_page() {
        let mut st = state();
        st.set_product(emb_model::ProductKind::TShirt);
        st.edit_product_for_test(|v| v.both_sides = true);
        let pages = |pdf: &[u8]| {
            let text: String = pdf.iter().map(|&b| if b < 128 { b as char } else { '?' }).collect();
            (
                text.matches("/Type /Page ").count(),
                text.matches("/Subtype /Image").count(),
            )
        };
        for paper in [Paper::A4, Paper::Letter] {
            for with in [true, false] {
                st.settings.output = OutputProfile {
                    worksheet: false,
                    approval: true,
                    approval_product: with,
                    picture: false,
                    paper,
                    ..OutputProfile::default()
                };
                let dir = temp(&format!("approval-{with}-{paper:?}"));
                output(&mut st, &dir, "shirt").unwrap();
                let pdf = std::fs::read(dir.join("shirt approval.pdf")).unwrap();
                // The picture of the design and its soft mask (ADR 0130), and
                // both sides: two pictures on the product page.
                assert_eq!(pages(&pdf), if with { (2, 4) } else { (1, 2) }, "{paper:?}");
                std::fs::remove_dir_all(&dir).ok();
            }
        }
        // The preview shows the same pages and keeps the picture.
        st.settings.output.approval_product = true;
        let first = st.preview_pages(true);
        assert_eq!(first.len(), 2);
        let pictures = |p: &Page| -> Vec<(emb_geom::BBox, *const emb_render::raster::Picture)> {
            p.ops
                .iter()
                .filter_map(|o| match o {
                    Op::Image { rect, picture } => Some((*rect, std::sync::Arc::as_ptr(picture))),
                    _ => None,
                })
                .collect()
        };
        let shown = pictures(&first[1]);
        assert_eq!(shown.len(), 2);
        assert_eq!(pictures(&st.preview_pages(true)[1])[0].1, shown[0].1);
        // The front is over the back, both in the margins of the page.
        let (front, back) = (shown[0].0, shown[1].0);
        assert!(back.min.y > front.max.y);
        for r in [front, back] {
            assert!(r.min.x >= 14.9 && r.max.x <= first[1].width_mm - 14.9, "{r:?}");
            assert!(r.max.y < first[1].height_mm - 15.0, "{r:?}");
        }
        // No product: no product page.
        st.doc.edit("None", |d| d.product = None);
        assert_eq!(st.preview_pages(true).len(), 1);
    }

    /// Writes approval sheets with the product page on A4 and Letter:
    /// `EMB_PRODUCT_OUT=<folder> cargo test -p emb-app -- --ignored write_approval_sheets`.
    #[test]
    #[ignore]
    fn write_approval_sheets() {
        let dir = PathBuf::from(std::env::var("EMB_PRODUCT_OUT").expect("set EMB_PRODUCT_OUT"));
        let mut st = state();
        st.set_product(emb_model::ProductKind::Polo);
        st.edit_product_for_test(|v| {
            v.both_sides = true;
            v.color = emb_model::Rgb(30, 40, 75);
        });
        for paper in [Paper::A4, Paper::Letter] {
            st.settings.output = OutputProfile {
                worksheet: false,
                approval: true,
                picture: false,
                paper,
                ..OutputProfile::default()
            };
            let out = dir.join(format!("{paper:?}"));
            output(&mut st, &out, "approval").unwrap();
        }
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
        assert!(lines[0].as_ref().is_ok_and(|l| l.ends_with("4 files")), "{lines:?}");
        assert!(lines[1].as_ref().is_ok_and(|l| l.ends_with("4 files")), "{lines:?}");
        assert!(lines[2].as_ref().is_err_and(|l| l.contains("not written")), "{lines:?}");
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

/// A worksheet in Japanese as a PDF, to look at in a PDF reader: the
/// Japanese text is in a font of the reader (`emb_render::pdf`).
#[cfg(test)]
mod japanese_sheet {
    use emb_model::{HoopCatalog, MachineCatalog};

    use crate::app::AppState;

    fn pdf() -> Vec<u8> {
        crate::i18n::set_language("ja");
        let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
        st.start_on_machine("husqvarna_topaz20", None);
        st.add_sample_objects();
        let pages = st.preview_pages(false);
        crate::i18n::set_language("en");
        emb_render::pdf::write(&pages, "生産作業シート")
    }

    #[test]
    fn the_japanese_worksheet_has_its_text_in_the_japanese_font() {
        let pdf = String::from_utf8_lossy(&pdf()).into_owned();
        assert!(pdf.contains("/UniJIS-UCS2-H"), "the Japanese font");
        // ステッチ (stitches), as the numbers of its characters.
        assert!(pdf.contains("30B930C630C330C1"), "a Japanese label");
        // No line of text is all question marks.
        assert!(!pdf.contains("(????"), "text that the PDF cannot show");
    }

    /// Run it by hand, then open `target/window-pictures/worksheet.ja.pdf`.
    #[test]
    #[ignore = "writes a PDF to look at"]
    fn write_japanese_worksheet() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("target")
            .join("window-pictures");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("worksheet.ja.pdf"), pdf()).unwrap();
    }
}
