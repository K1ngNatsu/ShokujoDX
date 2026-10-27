//! Command-line tools: convert machine files, stitch designs, and write the
//! test corpus. The `emb-cli` binary is a thin wrapper around these functions.
#![forbid(unsafe_code)]

pub mod bench;
pub mod corpus;

use std::path::Path;

use emb_model::{Design, MachineCatalog, StitchDesign, StitchKind, project};
use emb_stitch::{PlanOptions, StitchCache, StitchPlan, build_plan};

/// A file that was read: a design with objects, or a machine file with stitches only.
#[derive(Debug)]
pub enum Input {
    Design(Box<Design>),
    Stitches(StitchDesign),
}

fn extension(path: &Path) -> String {
    path.extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}

fn stem(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "design".into())
}

/// Reads a project file (`.embproj`), artwork (`.svg`) or a machine file
/// (`emb_formats::READ_EXTENSIONS`).
pub fn read_input(path: &Path) -> Result<Input, String> {
    let ext = extension(path);
    if ext == project::FILE_EXTENSION {
        return project::load(path)
            .map(|d| Input::Design(Box::new(d)))
            .map_err(|e| e.to_string());
    }
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    match ext.as_str() {
        "svg" => {
            let import = emb_formats::svg::import_svg(&bytes).map_err(|e| e.to_string())?;
            for w in &import.warnings {
                eprintln!("warning: {w}");
            }
            Ok(Input::Design(Box::new(import.to_design(&stem(path)))))
        }
        e if emb_formats::READ_EXTENSIONS.contains(&e) => emb_library::read_machine_file(path)
            .map(Input::Stitches)
            .map_err(|e| format!("{}: {e}", path.display())),
        _ => Err(format!(
            "{}: unknown input type (use .embproj, .svg or .{})",
            path.display(),
            emb_formats::READ_EXTENSIONS.join(", .")
        )),
    }
}

/// Sets the fabric of a design (see `emb_model::fabric`): the satin and
/// fill objects change with it. Machine files have no stitch settings; the
/// fabric changes nothing for them.
pub fn set_fabric(input: &mut Input, id: &str) -> Result<(), String> {
    let catalog = emb_model::FabricCatalog::builtin();
    if catalog.get(id).is_none() {
        let ids: Vec<&str> = catalog.fabrics.iter().map(|f| f.id.as_str()).collect();
        return Err(format!("unknown fabric {id:?} (use one of: {})", ids.join(", ")));
    }
    match input {
        Input::Design(d) => {
            d.set_fabric(&catalog, Some(id), true);
            Ok(())
        }
        Input::Stitches(_) => Err("--fabric needs a project or SVG file: a machine file has no stitch settings".into()),
    }
}

/// The fabric presets as lines of text: id, name, backing and topping.
pub fn fabric_list() -> String {
    let mut out = String::new();
    for f in emb_model::FabricCatalog::builtin().fabrics {
        out.push_str(&format!(
            "{:<8} {}
         backing: {}
",
            f.id, f.name, f.backing
        ));
        if let Some(t) = &f.topping {
            out.push_str(&format!(
                "         topping: {t}
"
            ));
        }
    }
    out
}

/// Stitch plan options: the machine `machine`, else the design's machine, else the defaults.
pub fn plan_options(design: &Design, machine: Option<&str>) -> Result<PlanOptions, String> {
    let catalog = MachineCatalog::builtin();
    match machine.or(design.machine.as_deref()) {
        Some(id) => catalog
            .get(id)
            .map(PlanOptions::for_machine)
            .ok_or_else(|| format!("unknown machine {id:?}")),
        None => Ok(PlanOptions::default()),
    }
}

pub fn plan(design: &Design, opts: &PlanOptions) -> StitchPlan {
    build_plan(design, opts, &mut StitchCache::default())
}

/// Stitches of an input: the stitch plan of a design, or the stitches of a machine file.
pub fn stitches(input: &Input, machine: Option<&str>) -> Result<(StitchDesign, Vec<String>), String> {
    match input {
        Input::Design(d) => {
            let p = plan(d, &plan_options(d, machine)?);
            let warnings = p.warnings.iter().map(|w| w.message.clone()).collect();
            Ok((p.stitches, warnings))
        }
        Input::Stitches(s) => Ok((s.clone(), Vec::new())),
    }
}

/// Writes a machine file. The type comes from the extension
/// (`emb_formats::WRITE_EXTENSIONS`). `.json` writes the stitch list as JSON,
/// for checks in other tools; `.svg` writes the design picture.
pub fn write_output(s: &StitchDesign, path: &Path) -> Result<(), String> {
    write_output_for(s, path, None)
}

/// As [`write_output`], with the encoding settings of the machine (the trim
/// code of DST and TAP files, and the needles of a multi-needle machine
/// with no pins).
pub fn write_output_for(
    s: &StitchDesign,
    path: &Path,
    machine: Option<&emb_model::MachineProfile>,
) -> Result<(), String> {
    let needles = machine.and_then(|m| emb_model::needles::plan_for(&emb_model::Design::new(""), s, m, None));
    write_output_with(s, path, machine, needles.as_ref())
}

/// As [`write_output_for`], with a needle plan (ADR 0063).
pub fn write_output_with(
    s: &StitchDesign,
    path: &Path,
    machine: Option<&emb_model::MachineProfile>,
    needles: Option<&emb_model::NeedlePlan>,
) -> Result<(), String> {
    let ext = extension(path);
    let date = chrono::Local::now().format("%Y%m%d%H%M%S").to_string();
    let bytes = match ext.as_str() {
        "json" => serde_json::to_vec_pretty(s).map_err(|e| e.to_string())?,
        "svg" => emb_render::svg::picture(&emb_render::segments(s), &[], 0.4, 2.0, None).into_bytes(),
        e => emb_formats::write_machine_with(e, s, &stem(path), &date, machine, needles).ok_or_else(|| {
            format!(
                "{}: unknown output type (use .{}, .json or .svg)",
                path.display(),
                emb_formats::WRITE_EXTENSIONS.join(", .")
            )
        })?,
    };
    std::fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))
}

/// FNV-1a hash of all records, with positions rounded to 0.001 mm. Golden
/// tests compare it, so that any change of the stitches shows.
pub fn fingerprint(s: &StitchDesign) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |bytes: &[u8]| {
        for b in bytes {
            h ^= u64::from(*b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    };
    for r in &s.records {
        eat(&[r.kind as u8]);
        eat(&((r.pos.x * 1000.0).round() as i64).to_le_bytes());
        eat(&((r.pos.y * 1000.0).round() as i64).to_le_bytes());
    }
    for t in &s.threads {
        eat(&[t.color.0, t.color.1, t.color.2]);
    }
    h
}

/// Longest stitch between two needle points sewn one after another, in mm.
pub fn longest_stitch(s: &StitchDesign) -> f64 {
    let mut longest: f64 = 0.0;
    let mut prev = None;
    for r in &s.records {
        match r.kind {
            StitchKind::Stitch => {
                if let Some(p) = prev {
                    longest = longest.max(r.pos.distance(p));
                }
                prev = Some(r.pos);
            }
            _ => prev = None,
        }
    }
    longest
}

/// A short text summary of a stitch list, the same on all platforms.
pub fn summary(s: &StitchDesign) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "stitches: {}\njumps: {}\ntrims: {}\ncolors: {}\n",
        s.stitch_count(),
        s.jump_count(),
        s.trim_count(),
        s.color_count()
    ));
    if let Some(b) = s.extents() {
        out.push_str(&format!(
            "extents: x {:.1} .. {:.1}, y {:.1} .. {:.1} (size {:.1} x {:.1} mm)\n",
            b.min.x,
            b.max.x,
            b.min.y,
            b.max.y,
            b.width(),
            b.height()
        ));
    }
    out.push_str(&format!("longest stitch: {:.2} mm\n", longest_stitch(s)));
    let colors: Vec<String> = s.threads.iter().map(|t| t.color.to_hex()).collect();
    out.push_str(&format!("threads: {}\n", colors.join(" ")));
    out.push_str(&format!("fingerprint: {:016x}\n", fingerprint(s)));
    out
}

/// The thread use and the density of `s` (P11 Milestone 1), for
/// `emb-cli stitch`. Not in [`summary`]: the golden snapshots are the summary.
pub fn production(s: &StitchDesign, thickness_mm: f64) -> String {
    use emb_model::thread_use::{length_text, thread_use};
    let u = thread_use(s, thickness_mm);
    let colors: Vec<String> = u.colors.iter().map(|c| length_text(c.top_mm)).collect();
    let mut out = format!(
        "top thread: {} ({})
bobbin thread: {}
",
        length_text(u.top_mm),
        colors.join(", "),
        length_text(u.bobbin_mm)
    );
    if let Some(m) = emb_render::density::DensityMap::of(s) {
        let limit = emb_render::density::DEFAULT_LIMIT;
        let spots = m.hot_spots(limit);
        out.push_str(&format!(
            "density: up to {:.1} layers, {} places over {limit:.1}
",
            m.max_layers(),
            spots.len()
        ));
        for h in spots.iter().take(10) {
            out.push_str(&format!(
                "  {:.1} layers on {:.0} mm² at {:.1}, {:.1}
",
                h.max_layers, h.area_mm2, h.peak.x, h.peak.y
            ));
        }
    }
    out
}

/// The settings of `emb-cli quote` (ADR 0064).
#[derive(Debug, Clone, Default)]
pub struct QuoteOptions {
    pub list: emb_model::quote::PriceList,
    pub order: emb_model::quote::Order,
    pub currency: String,
    /// The heads of the machine, if not the heads of its profile.
    pub heads: Option<u32>,
}

/// The quote of an order of `input` (ADR 0064) and its run time on the
/// machine (`None` with no machine). The sew time of one piece has the
/// thread changes of a multi-needle machine.
pub fn quote(
    input: &Input,
    machine: Option<&str>,
    o: &QuoteOptions,
) -> Result<(emb_model::quote::Quote, Option<emb_model::quote::RunTime>, StitchDesign), String> {
    use emb_model::quote::{Piece, quote, run_time};
    let (s, _) = stitches(input, machine)?;
    let q = quote(&o.list, Piece::of(&s), &o.order);
    let catalog = MachineCatalog::builtin();
    let id = machine.or(match input {
        Input::Design(d) => d.machine.as_deref(),
        Input::Stitches(_) => None,
    });
    let run = id.and_then(|id| catalog.get(id)).map(|m| {
        let mut t = m.sew_times();
        if let Some(h) = o.heads {
            t.heads = h.clamp(1, emb_model::machine::MAX_HEADS);
        }
        let changes = match input {
            Input::Design(d) => emb_model::needles::plan_for(d, &s, m, None).map_or(0, |p| p.changes.len()),
            Input::Stitches(_) => 0,
        };
        run_time(&t, t.minutes_of(&s, changes), q.quantity)
    });
    Ok((q, run, s))
}

/// A quote as text: one line for each line of the price, the total, and
/// the run time.
pub fn quote_text(q: &emb_model::quote::Quote, run: Option<&emb_model::quote::RunTime>, currency: &str) -> String {
    use emb_model::quote::{duration_text, money_text};
    let mut out = String::new();
    for l in &q.lines {
        let each = if l.kind.per_piece() {
            format!("{} × {}", l.quantity, money_text(currency, l.each))
        } else {
            String::new()
        };
        out.push_str(&format!(
            "{:<36}{:>18}{:>14}
",
            l.kind.label(),
            each,
            money_text(currency, l.amount)
        ));
    }
    out.push_str(&format!(
        "{:<54}{:>14}
",
        "Total",
        money_text(currency, q.total)
    ));
    out.push_str(&format!(
        "{} pieces, {} for each piece
",
        q.quantity,
        money_text(currency, q.each)
    ));
    if let Some(r) = run {
        out.push_str(&format!(
            "run time: {} on {} heads, {} cycles, {:.1} pieces in an hour{}
",
            duration_text(r.minutes),
            r.heads,
            r.cycles,
            r.pieces_per_hour,
            if r.operator_bound {
                " (the operator sets the speed)"
            } else {
                ""
            }
        ));
    }
    out
}

/// Parses `--set name=value` for a script: a number, `true`, `false`, or a
/// text.
pub fn script_input(s: &str) -> Result<(String, emb_script::InputValue), String> {
    use emb_script::InputValue;
    let (k, v) = s.split_once('=').ok_or("--set needs name=value")?;
    let value = match v.trim() {
        "true" => InputValue::Bool(true),
        "false" => InputValue::Bool(false),
        t => match t.parse::<f64>() {
            Ok(n) if n.is_finite() => InputValue::Number(n),
            _ => InputValue::Text(t.to_string()),
        },
    };
    Ok((k.trim().to_string(), value))
}

/// Runs script `src` on `input` (a machine file becomes a design first)
/// with the inputs `inputs` (ADR 0067). Returns the outcome; the caller
/// saves the design if it changed.
pub fn run_script(
    src: &str,
    input: Input,
    name: &str,
    inputs: &std::collections::BTreeMap<String, emb_script::InputValue>,
    machine: Option<&str>,
) -> Result<emb_script::Outcome, String> {
    let mut design = match input {
        Input::Design(d) => *d,
        Input::Stitches(s) => emb_model::design_from_stitches(&s, name),
    };
    if let Some(m) = machine {
        design.machine = Some(m.to_string());
    }
    let catalog = MachineCatalog::builtin();
    let profile = design.machine.as_deref().and_then(|m| catalog.get(m));
    emb_script::run(src, &design, &[], inputs, profile, &emb_script::Limits::default()).map_err(|e| e.to_string())
}

/// The lace check of a free-standing lace design (`emb_stitch::lace`): the
/// loose parts, or `None` if the design is not lace.
pub fn lace_report(design: &Design, machine: Option<&str>) -> Result<Option<String>, String> {
    if !emb_stitch::lace::is_lace(design, &emb_model::FabricCatalog::builtin()) {
        return Ok(None);
    }
    let p = plan(design, &plan_options(design, machine)?);
    let loose = emb_stitch::lace::loose_parts(design, &p);
    let mut out = format!(
        "lace: {} loose parts
",
        loose.len()
    );
    for l in &loose {
        out.push_str(&format!(
            "  object {}: {:.0} mm of thread at {:.1}, {:.1}
",
            l.object.0, l.length_mm, l.center.x, l.center.y
        ));
    }
    Ok(Some(out))
}

/// What `output` writes for each design.
#[derive(Debug, Clone, PartialEq)]
pub struct OutputOptions {
    /// Machine formats by extension. Empty: the default format of the machine.
    pub formats: Vec<String>,
    pub worksheet: bool,
    pub approval: bool,
    pub picture: bool,
    /// The design picture as SVG.
    pub svg: bool,
    pub paper: emb_render::page::Paper,
    /// A fabric id for all designs. For a machine file it is only shown on the sheets.
    pub fabric: Option<String>,
    /// The barcode (the file name) on the production worksheet.
    pub barcode: emb_render::barcode::BarcodeKind,
    /// For a design with rhinestones: the template (SVG and PDF), and the
    /// clearance of its holes in mm.
    pub stone_template: bool,
    pub stone_clearance_mm: f64,
    /// The PNG picture with the flat thread look (else the 3D thread look,
    /// with the texture of the fabric under a background color).
    pub flat_picture: bool,
}

/// Writes the output files of one input (a project, artwork or machine file)
/// to `dir` as `<name>.*`: machine files, the project file, and the sheets
/// and picture that `o` asks for. Returns the files and the sheet data (for CSV).
pub fn output(
    input: &Path,
    dir: &Path,
    machine: Option<&str>,
    o: &OutputOptions,
) -> Result<(Vec<std::path::PathBuf>, emb_render::sheet::SheetData), String> {
    use emb_render::sheet;
    let name = stem(input);
    let mut input = match read_input(input)? {
        Input::Stitches(s) => Input::Design(Box::new(emb_model::design_from_stitches(&s, &name))),
        i => i,
    };
    if let Some(f) = &o.fabric {
        set_fabric(&mut input, f)?;
    }
    let Input::Design(design) = input else {
        unreachable!("machine files are designs here")
    };
    let mut design = *design;
    if let Some(m) = machine {
        design.machine = Some(m.to_string());
    }
    let opts = plan_options(&design, None)?;
    let p = plan(&design, &opts);
    let catalog = MachineCatalog::builtin();
    let profile = design.machine.as_deref().and_then(|m| catalog.get(m));
    let mut formats: Vec<String> = o.formats.clone();
    if formats.is_empty() {
        formats.push(profile.map_or("dst".into(), |m| m.default_format.clone()));
    }
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut files = Vec::new();
    // A design of rhinestones only has no machine file.
    let needles = profile.and_then(|m| emb_model::needles::plan_for(&design, &p.stitches, m, None));
    for ext in formats.iter().filter(|_| p.stitches.stitch_count() > 0) {
        let path = dir.join(format!("{name}.{ext}"));
        write_output_with(&p.stitches, &path, profile, needles.as_ref())?;
        files.push(path);
    }
    let proj = dir.join(format!("{name}.{}", project::FILE_EXTENSION));
    project::save(&design, &proj).map_err(|e| e.to_string())?;
    files.push(proj);

    let mut data = sheet::SheetData::from_stitches(&design.name, &p.stitches);
    if let Some(plan) = &needles {
        data.set_needles(plan);
    }
    data.machine = profile.map(|m| m.name.clone());
    data.minutes = profile.map(|m| {
        m.sew_times().minutes(
            data.stitches,
            data.colors.len().saturating_sub(1),
            data.trims,
            data.stops,
        )
    });
    let hoop = match &design.hoop.selection {
        emb_model::HoopSelection::Preset { id } => {
            Some((id.clone(), emb_model::HoopCatalog::builtin().get(id).cloned()))
        }
        emb_model::HoopSelection::OpenField => None,
    };
    data.hoop = hoop
        .as_ref()
        .map(|(id, h)| h.as_ref().map_or(id.clone(), |h| h.name.clone()));
    data.hoop_layout = hoop
        .and_then(|(_, h)| h)
        .and_then(|h| sheet::HoopLayout::new(&h, &design, &p.stitches));
    if let Some(f) = design
        .fabric
        .as_deref()
        .and_then(|id| emb_model::FabricCatalog::builtin().get(id).cloned())
    {
        data.set_thread_use(&p.stitches, f.thickness_mm);
        data.fabric = Some(f.name);
        data.backing = Some(f.backing);
        data.topping = f.topping;
    }
    data.stop_notes = p.stop_notes(&design);
    data.fabric_areas = sheet::fabric_areas(&design);
    (data.stones, data.stone_counts) = sheet::stones_of(&design);
    data.notes = design.notes.clone();
    data.order = design.order.clone();
    data.background = design.background;
    if o.stone_template && design.stone_template(o.stone_clearance_mm).is_some() {
        for ext in ["svg", "pdf"] {
            let p = dir.join(format!("{name} template.{ext}"));
            write_stone_template(&design, &p, o.stone_clearance_mm, o.paper)?;
            files.push(p);
        }
    }
    data.process_steps = sheet::process_steps(&design, &name);
    data.date = chrono::Local::now().format("%Y-%m-%d").to_string();
    data.files = files
        .iter()
        .filter_map(|f| f.file_name().map(|n| n.to_string_lossy().into_owned()))
        .collect();
    data.barcode = (o.barcode != emb_render::barcode::BarcodeKind::None).then(|| (o.barcode, data.barcode_text(&name)));
    let mut write = |path: std::path::PathBuf, bytes: Vec<u8>| -> Result<(), String> {
        std::fs::write(&path, bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        files.push(path);
        Ok(())
    };
    if o.worksheet {
        write(
            dir.join(format!("{name} worksheet.pdf")),
            emb_render::pdf::write(&sheet::production(&data, o.paper), &design.name),
        )?;
    }
    if o.approval {
        write(
            dir.join(format!("{name} approval.pdf")),
            emb_render::pdf::write(&[sheet::approval(&data, o.paper)], &design.name),
        )?;
    }
    if o.picture {
        let areas = sheet::picture_areas(&design);
        let look = picture_look(&design, o.flat_picture);
        let pic = emb_render::raster::picture_look(&data.segments, &areas, 10.0, 0.4, 2.0, design.background, &look);
        write(dir.join(format!("{name}.png")), pic.png())?;
    }
    if o.svg {
        let svg = emb_render::svg::picture(
            &data.segments,
            &sheet::picture_areas(&design),
            0.4,
            2.0,
            design.background,
        );
        write(dir.join(format!("{name}.svg")), svg.into_bytes())?;
    }
    Ok((files, data))
}

/// The look of the design picture: the 3D thread look on the fabric of the
/// design (P10 Milestone 3), or the flat look.
pub fn picture_look(design: &Design, flat: bool) -> emb_render::raster::Look {
    if flat {
        return emb_render::raster::Look::default();
    }
    emb_render::raster::Look::three_d(Some(emb_render::thread3d::Weave::for_fabric(design.fabric.as_deref())))
}

/// Writes the rhinestone template of `design` (P9 Milestone 4): SVG or DXF
/// for a cutter, or PDF (the key and the template at full size), by the
/// extension of `path`. Returns the number of holes.
pub fn write_stone_template(
    design: &emb_model::Design,
    path: &Path,
    clearance_mm: f64,
    paper: emb_render::page::Paper,
) -> Result<usize, String> {
    let t = design
        .stone_template(clearance_mm)
        .ok_or("the design has no rhinestones")?;
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    let bytes = match ext.as_str() {
        "svg" => emb_formats::stones::svg(&t).into_bytes(),
        "dxf" => emb_formats::stones::dxf(&t).into_bytes(),
        "pdf" => {
            let name = if design.name.is_empty() { "Design" } else { &design.name };
            let spec = emb_render::stone_sheet::StoneSheetSpec {
                name: name.to_string(),
                paper,
                clearance_mm,
            };
            emb_render::pdf::write(&emb_render::stone_sheet::pages(&t, &spec), name)
        }
        _ => return Err(format!("{}: use .svg, .dxf or .pdf", path.display())),
    };
    std::fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(t.holes.len())
}
