#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use emb_cli::{corpus, read_input, stitches, summary, write_output};

const USAGE: &str = "\
Usage:
  emb-cli convert <input> <output> [--machine <id>] [--fabric <id>]
      Stitch a design (.embproj, .svg) or read a machine file (.dst, .vp3,
      .pes, .pec, .exp, .jef, .xxx, .sew, .hus, .shv, .tbf, .tap, .dsb, .dsz,
      .10o, .100, .u01, .dat and the other formats of emb_formats; a file
      with no colors takes them from a .inf, .edr or .col file next to it),
      then write a machine file (.dst, .vp3, .pes, .pec, .exp, .jef, .xxx,
      .tbf, .tap, .u01), the stitch list as JSON (.json) or the design
      picture as SVG (.svg).
  emb-cli stitch <input> [--machine <id>] [--fabric <id>]
      Print a summary of the stitches of the input, the thread use (an
      estimate, on the fabric of the design), the places of the density
      map over 5 layers of thread and, for free-standing lace, the parts
      that touch no other part.
  emb-cli output <input>... --dir <dir> [--machine <id>] [--format vp3,dst]
                 [--worksheet] [--approval] [--picture] [--svg] [--csv] [--paper a4|letter]
                 [--fabric <id>] [--barcode none|code39|code128] [--clearance <mm>]
                 [--flat-picture]
      Batch output: for each input, the machine files (default: the format
      of the machine), the project file, and the worksheet (PDF), approval
      sheet (PDF), picture (PNG) and SVG picture when asked. --csv writes designs.csv
      with one line for each design. The PNG picture has the 3D thread look
      (--flat-picture: the flat look).
  emb-cli library <folder>... [--search <text>] [--sort name|folder|date|size|stitches|colors|format]
                 [--desc] [--index <file.json>] [--list <file.csv>] [--catalog <file.pdf>]
      Find the designs in the folders and their subfolders (ADR 0035) and
      print the designs that the search text finds (words, and filters
      such as format:vp3 colors:<4 width:<100 fits:topaz20_crown). With
      --index the numbers are kept in that file (thumbnails next to it), so
      the next run reads only the changed files. --list writes the design
      list as CSV, --catalog the catalog (PDF, 20 designs on a page).
  emb-cli cutter <design.embproj> <out.svg|out.dxf>
      Write the cut lines of the appliqué pieces at real size, for a fabric
      cutting machine.
  emb-cli bling <design.embproj> <out.svg|out.dxf|out.pdf> [--clearance <mm>] [--paper a4|letter]
      Write the rhinestone template: a hole for each stone (the stone
      size plus the clearance, 0.3 mm by default) and registration marks,
      at real size, for a cutter (SVG, DXF), or on paper (PDF).
  emb-cli chart <design.embproj> <out.pdf> [--paper a4|letter]
      Write the chart of the cross-stitch objects (a symbol for each
      color, the key, the size on Aida fabric) for hand cross stitch.
  emb-cli bench [<folder>] [--width <mm>] [--detail <mm>] [--colors <n>]
                 [--specks <mm2>] [--smooth <n>] [--denoise <n>]
                 [--background keep|edge] [--out <folder>]
      Run the steps of Picture to design on the test pictures of the
      program and on the pictures in the folder (the edge color removed or
      kept; 100 mm wide, 6 colors and the start settings of the wizard, or
      the settings given), and print for each picture the objects,
      colors, stitches and trims, how much of the picture the stitches
      cover, the stitches outside it, the mean color difference (ΔE) and
      the time of each step. --out writes a picture of the stitches of
      each picture to the folder.
  emb-cli quote <input> [--prices <list.toml>] [--quantity <n>] [--new-design] [--rush]
                [--garment <price>] [--currency <text>] [--machine <id>] [--heads <n>]
                [--sheet <out.pdf>] [--paper a4|letter]
      Print the price of an order of the design (ADR 0064) and the run time
      on the machine. The price list is a TOML file with the fields of a
      price list (per_1000, min_piece, free_colors, per_color, setup,
      digitizing, digitizing_per_1000, rush_percent, tax_percent, and
      [[breaks]] with from and percent_off). With no --quantity, --new-design,
      --rush or --garment, the order of the design (Design > Order Details)
      gives them. --sheet writes the quote sheet (PDF) for the customer.
  emb-cli script <script.rhai> <input>... [--set <name>=<value>]... [--out <dir>]
               [--machine <id>]
      Run a script (ADR 0067, Help > Scripts) on each input (a project file,
      an SVG or a machine file) and print its messages. With --out, the
      changed designs are saved there as project files (the same name);
      with no --out, nothing is saved. --set gives an input of the script.
  emb-cli fabrics
      List the fabric presets. --fabric changes the density, pull
      compensation and underlay of the satin and fill objects for the fabric.
  emb-cli corpus <dir>
      Write the test corpus to <dir>: a project file and VP3, DST and PES
      files for each design, and for a design in several hoopings also
      <name>_1.vp3, <name>_2.vp3, ... (one for each hooping).
  emb-cli font <font.ttf> <out.embfont> --name <family> [--style <style>]
               [--stitch auto|satin|fill] [--small-text]
      Convert a TrueType or OpenType font, or a bitmap font (.bdf, .fon:
      the largest size), to an embroidery font (ADR 0016, 0021).
      --small-text: a font for letters 4 to 6 mm high (satin columns from
      0.8 mm).
      Check that the license of the font lets you do this; an OFL font
      with a Reserved Font Name needs a new --name.";

/// The files of `emb-cli library`.
struct LibraryFiles<'a> {
    index: Option<&'a Path>,
    list: Option<&'a Path>,
    catalog: Option<&'a Path>,
    paper: emb_render::page::Paper,
}

/// `emb-cli library`: scan the folders, search, sort and print.
fn library(
    folders: &[&str],
    search: &str,
    key: emb_library::SortKey,
    desc: bool,
    files: &LibraryFiles,
) -> Result<(), String> {
    let (index_file, list) = (files.index, files.list);
    use std::time::Instant;
    let roots: Vec<PathBuf> = folders
        .iter()
        .map(|f| std::fs::canonicalize(f).map_err(|e| format!("{f}: {e}")))
        .collect::<Result<_, _>>()?;
    let old = index_file.map(emb_library::Index::load).unwrap_or_default();
    let thumbs = index_file.and_then(Path::parent).map(|d| d.join("thumbnails"));
    let t = Instant::now();
    let (index, report) = emb_library::scan(&roots, &old, thumbs.as_deref(), |_, _| true);
    let scan_s = t.elapsed().as_secs_f64();
    if let Some(f) = index_file {
        index.save(f).map_err(|e| format!("{}: {e}", f.display()))?;
    }
    let hoops = emb_model::HoopCatalog::builtin();
    let t = Instant::now();
    let q = emb_library::Query::parse(search, &hoops);
    let mut found = q.run(&index);
    emb_library::sort(&index, &mut found, key, desc);
    let search_ms = t.elapsed().as_secs_f64() * 1000.0;
    for p in q.problems() {
        eprintln!("warning: {p}");
    }
    for &i in &found {
        let e = &index.entries[i];
        match &e.error {
            Some(err) => println!("{} [{}] cannot be read: {err}", e.name, e.format),
            None => println!(
                "{} [{}] {:.0} x {:.0} mm, {} stitches, {} color{}{}{}",
                e.name,
                e.format,
                e.width_mm,
                e.height_mm,
                e.stitches,
                e.colors,
                if e.colors == 1 { "" } else { "s" },
                if e.folder.is_empty() { "" } else { " in " },
                e.folder
            ),
        }
    }
    println!(
        "{} of {} designs ({} read, {} kept, {} removed, {} cannot be read); scan {scan_s:.2} s, search {search_ms:.1} ms",
        found.len(),
        index.entries.len(),
        report.read,
        report.kept,
        report.removed,
        report.failed
    );
    if let Some(f) = list {
        std::fs::write(f, emb_library::csv::design_list(&index, &found))
            .map_err(|e| format!("{}: {e}", f.display()))?;
        println!("wrote {}", f.display());
    }
    if let Some(f) = files.catalog {
        let items: Vec<emb_library::catalog::Item> = found
            .iter()
            .map(|&i| {
                let e = &index.entries[i];
                emb_library::catalog::Item {
                    entry: e,
                    stitches: emb_library::read_stitches(&e.path).ok(),
                }
            })
            .collect();
        let title = if search.is_empty() {
            "Design catalog".to_string()
        } else {
            format!("Design catalog: {search}")
        };
        let pages = emb_library::catalog::pages(&title, &items, files.paper);
        std::fs::write(f, emb_render::pdf::write(&pages, &title)).map_err(|e| format!("{}: {e}", f.display()))?;
        println!("wrote {} ({} pages)", f.display(), pages.len());
    }
    Ok(())
}

fn run(args: &[String]) -> Result<(), String> {
    let mut machine = None;
    let mut dir = None;
    let mut out = emb_cli::OutputOptions {
        formats: Vec::new(),
        worksheet: false,
        approval: false,
        picture: false,
        svg: false,
        paper: emb_render::page::Paper::Letter,
        fabric: None,
        barcode: emb_render::barcode::BarcodeKind::Code128,
        stone_template: true,
        stone_clearance_mm: emb_model::bling::DEFAULT_CLEARANCE_MM,
        flat_picture: false,
    };
    let mut csv = false;
    let (mut search, mut sort_key, mut desc, mut index_file, mut list, mut catalog) =
        (String::new(), emb_library::SortKey::Name, false, None, None, None);
    let (mut name, mut style, mut stitch) = (None, String::new(), emb_model::LetterStitch::Auto);
    let mut small_text = false;
    let mut bench = emb_cli::bench::FolderSettings::default();
    let mut bench_out: Option<PathBuf> = None;
    let mut quote = emb_cli::QuoteOptions::default();
    // An order option on the command line; else the order of the design.
    let mut order_given = false;
    let mut sheet: Option<PathBuf> = None;
    let mut script_inputs = std::collections::BTreeMap::new();
    let mut rest = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--machine" => machine = Some(it.next().ok_or("--machine needs a machine id")?.as_str()),
            "--dir" => dir = Some(PathBuf::from(it.next().ok_or("--dir needs a folder")?)),
            "--format" => {
                out.formats = it
                    .next()
                    .ok_or("--format needs a list such as vp3,dst")?
                    .split(',')
                    .map(|s| s.trim().to_lowercase())
                    .collect()
            }
            "--worksheet" => out.worksheet = true,
            "--approval" => out.approval = true,
            "--picture" => out.picture = true,
            "--svg" => out.svg = true,
            "--flat-picture" => out.flat_picture = true,
            "--csv" => csv = true,
            "--clearance" => {
                out.stone_clearance_mm = it
                    .next()
                    .and_then(|v| v.parse::<f64>().ok())
                    .filter(|v| (0.0..=3.0).contains(v))
                    .ok_or("--clearance needs a number of mm from 0 to 3")?
            }
            "--search" => search = it.next().ok_or("--search needs a text")?.clone(),
            "--sort" => {
                let w = it.next().ok_or("--sort needs a key")?;
                sort_key = emb_library::SortKey::from_word(w).ok_or_else(|| format!("--sort: no key {w:?}"))?;
            }
            "--desc" => desc = true,
            "--index" => index_file = Some(PathBuf::from(it.next().ok_or("--index needs a file")?)),
            "--list" => list = Some(PathBuf::from(it.next().ok_or("--list needs a file")?)),
            "--catalog" => catalog = Some(PathBuf::from(it.next().ok_or("--catalog needs a file")?)),
            "--name" => name = Some(it.next().ok_or("--name needs a font name")?.clone()),
            "--style" => style = it.next().ok_or("--style needs a style name")?.clone(),
            "--small-text" => small_text = true,
            "--background" => {
                bench.background = match it.next().map(|s| s.to_lowercase()).as_deref() {
                    Some("keep") => emb_cli::bench::Background::Keep,
                    Some("edge") => emb_cli::bench::Background::EdgeColor,
                    _ => return Err("--background needs keep or edge".into()),
                }
            }
            "--prices" => {
                let f = it.next().ok_or("--prices needs a TOML file")?;
                let text = std::fs::read_to_string(f).map_err(|e| format!("{f}: {e}"))?;
                quote.list = emb_model::quote::PriceList::from_toml_str(&text).map_err(|e| format!("{f}: {e}"))?;
            }
            "--quantity" => {
                order_given = true;
                quote.order.quantity = it
                    .next()
                    .and_then(|v| v.parse::<u32>().ok())
                    .filter(|v| (1..=emb_model::quote::MAX_QUANTITY).contains(v))
                    .ok_or("--quantity needs a number of pieces (1 or more)")?
            }
            "--new-design" => {
                quote.order.new_design = true;
                order_given = true;
            }
            "--rush" => {
                quote.order.rush = true;
                order_given = true;
            }
            "--garment" => {
                order_given = true;
                quote.order.garment_price = it
                    .next()
                    .and_then(|v| v.parse::<f64>().ok())
                    .filter(|v| v.is_finite() && *v >= 0.0)
                    .ok_or("--garment needs a price")?
            }
            "--currency" => quote.currency = it.next().ok_or("--currency needs a text")?.clone(),
            "--heads" => {
                quote.heads = Some(
                    it.next()
                        .and_then(|v| v.parse::<u32>().ok())
                        .filter(|v| (1..=emb_model::machine::MAX_HEADS).contains(v))
                        .ok_or("--heads needs a number from 1 to 50")?,
                )
            }
            "--sheet" => sheet = Some(PathBuf::from(it.next().ok_or("--sheet needs a PDF file")?)),
            "--set" => {
                let (k, v) = emb_cli::script_input(it.next().ok_or("--set needs name=value")?)?;
                script_inputs.insert(k, v);
            }
            "--out" => bench_out = Some(PathBuf::from(it.next().ok_or("--out needs a folder")?)),
            "--width" | "--detail" | "--colors" | "--specks" | "--smooth" | "--denoise" => {
                let v = it
                    .next()
                    .and_then(|v| v.parse::<f64>().ok())
                    .filter(|v| *v >= 0.0)
                    .ok_or_else(|| format!("{a} needs a number"))?;
                let s = &mut bench.simplify;
                match a.as_str() {
                    "--width" => bench.width_mm = v.max(5.0),
                    "--detail" => s.pixel_mm = v.max(0.05),
                    "--colors" => s.colors = (v as usize).clamp(2, 32),
                    "--specks" => s.speck_mm2 = v,
                    "--smooth" => s.smooth = v as u32,
                    _ => s.denoise = v as u32,
                }
            }
            "--stitch" => {
                stitch = match it.next().map(|s| s.to_lowercase()).as_deref() {
                    Some("auto") => emb_model::LetterStitch::Auto,
                    Some("satin") => emb_model::LetterStitch::Satin,
                    Some("fill") => emb_model::LetterStitch::Fill,
                    _ => return Err("--stitch needs auto, satin or fill".into()),
                }
            }
            "--fabric" => {
                out.fabric = Some(
                    it.next()
                        .ok_or("--fabric needs a fabric id (see emb-cli fabrics)")?
                        .clone(),
                )
            }
            "--barcode" => {
                out.barcode = match it.next().map(|s| s.to_lowercase()).as_deref() {
                    Some("none") => emb_render::barcode::BarcodeKind::None,
                    Some("code39") => emb_render::barcode::BarcodeKind::Code39,
                    Some("code128") => emb_render::barcode::BarcodeKind::Code128,
                    _ => return Err("--barcode needs none, code39 or code128".into()),
                }
            }
            "--paper" => {
                out.paper = match it.next().map(|s| s.to_lowercase()).as_deref() {
                    Some("a4") => emb_render::page::Paper::A4,
                    Some("letter") => emb_render::page::Paper::Letter,
                    _ => return Err("--paper needs a4 or letter".into()),
                }
            }
            _ => rest.push(a.as_str()),
        }
    }
    match rest.as_slice() {
        ["convert", input, output] => {
            let (s, warnings) = stitches(&read_fabric(input, &out.fabric)?, machine)?;
            for w in warnings {
                eprintln!("warning: {w}");
            }
            let catalog = emb_model::MachineCatalog::builtin();
            emb_cli::write_output_for(&s, Path::new(output), machine.and_then(|m| catalog.get(m)))?;
            println!(
                "wrote {output}: {} stitches, {} colors",
                s.stitch_count(),
                s.color_count()
            );
            Ok(())
        }
        ["stitch", input] => {
            let input = read_fabric(input, &out.fabric)?;
            let fabrics = emb_model::FabricCatalog::builtin();
            let thickness = match (&out.fabric, &input) {
                (Some(id), _) => fabrics.get(id).map(|f| f.thickness_mm),
                (None, emb_cli::Input::Design(d)) => Some(fabrics.of(d).thickness_mm),
                _ => None,
            }
            .unwrap_or(emb_model::thread_use::DEFAULT_THICKNESS_MM);
            let (s, warnings) = stitches(&input, machine)?;
            for w in warnings {
                eprintln!("warning: {w}");
            }
            print!("{}", summary(&s));
            print!("{}", emb_cli::production(&s, thickness));
            if let emb_cli::Input::Design(d) = &input
                && let Some(lace) = emb_cli::lace_report(d, machine)?
            {
                print!("{lace}");
            }
            Ok(())
        }
        ["corpus", dir] => {
            let dir = PathBuf::from(dir);
            std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
            for name in corpus::NAMES {
                let design = corpus::build(name).expect("corpus names build");
                emb_model::project::save(&design, &dir.join(format!("{name}.embproj"))).map_err(|e| e.to_string())?;
                // A design in several hoopings: one file of each format for each hooping.
                let hoopings = design
                    .hooping
                    .as_ref()
                    .and_then(|l| corpus::hoop(&l.hoop))
                    .map(|h| design.hoopings(&h))
                    .unwrap_or_default();
                let (s, _) = stitches(&emb_cli::Input::Design(Box::new(design)), machine)?;
                for ext in ["vp3", "dst", "pes"] {
                    write_output(&s, &dir.join(format!("{name}.{ext}")))?;
                }
                println!("{name}: {} stitches, {} colors", s.stitch_count(), s.color_count());
                for (k, h) in hoopings.into_iter().enumerate() {
                    for w in &h.warnings {
                        eprintln!("warning: {w}");
                    }
                    let (s, _) = stitches(&emb_cli::Input::Design(Box::new(h.design)), machine)?;
                    for ext in ["vp3", "dst", "pes"] {
                        write_output(&s, &dir.join(format!("{name}_{}.{ext}", k + 1)))?;
                    }
                    println!(
                        "  hooping {}: {} stitches, {} colors",
                        k + 1,
                        s.stitch_count(),
                        s.color_count()
                    );
                }
            }
            Ok(())
        }
        ["output", inputs @ ..] if !inputs.is_empty() => {
            let dir = dir.ok_or("output needs --dir <folder>")?;
            let mut lines = vec![emb_render::sheet::csv_header().to_string()];
            let mut failed = 0;
            for input in inputs {
                match emb_cli::output(Path::new(input), &dir, machine, &out) {
                    Ok((files, data)) => {
                        println!("{input}: {} files", files.len());
                        lines.push(emb_render::sheet::csv_line(&data));
                    }
                    Err(e) => {
                        eprintln!("{input}: {e}");
                        failed += 1;
                    }
                }
            }
            if csv {
                let path = dir.join("designs.csv");
                std::fs::write(&path, lines.join("\n") + "\n").map_err(|e| format!("{}: {e}", path.display()))?;
                println!("wrote {}", path.display());
            }
            if failed > 0 {
                return Err(format!("{failed} of {} inputs failed", inputs.len()));
            }
            Ok(())
        }
        ["library", folders @ ..] if !folders.is_empty() => {
            let files = LibraryFiles {
                index: index_file.as_deref(),
                list: list.as_deref(),
                catalog: catalog.as_deref(),
                paper: out.paper,
            };
            library(folders, &search, sort_key, desc, &files)
        }
        ["cutter", input, output] => {
            let design = emb_model::project::load(Path::new(input)).map_err(|e| format!("{input}: {e}"))?;
            let pieces = emb_formats::cutter::pieces_of(&design, None);
            if pieces.is_empty() {
                return Err(format!("{input}: the design has no appliqué objects"));
            }
            let text = if output.to_lowercase().ends_with(".dxf") {
                emb_formats::cutter::dxf(&pieces)
            } else {
                emb_formats::cutter::svg(&pieces)
            };
            std::fs::write(output, text).map_err(|e| format!("{output}: {e}"))?;
            println!(
                "wrote {output}: {} piece{}",
                pieces.len(),
                if pieces.len() == 1 { "" } else { "s" }
            );
            Ok(())
        }
        ["bling", input, output] => {
            let design = emb_model::project::load(Path::new(input)).map_err(|e| format!("{input}: {e}"))?;
            let n = emb_cli::write_stone_template(&design, Path::new(output), out.stone_clearance_mm, out.paper)
                .map_err(|e| format!("{input}: {e}"))?;
            println!("wrote {output}: {n} holes");
            Ok(())
        }
        ["chart", input, output] => {
            let design = emb_model::project::load(Path::new(input)).map_err(|e| format!("{input}: {e}"))?;
            let (colors, cell_mm) = emb_render::cross_chart::colors_of(&design, None)
                .ok_or_else(|| format!("{input}: the design has no cross-stitch objects"))?;
            let title = if design.name.is_empty() {
                "Design".to_string()
            } else {
                design.name.clone()
            };
            let spec = emb_render::cross_chart::ChartSpec {
                title: title.clone(),
                paper: out.paper,
                cell_mm,
            };
            let pages = emb_render::cross_chart::chart(&colors, &spec);
            std::fs::write(output, emb_render::pdf::write(&pages, &title)).map_err(|e| format!("{output}: {e}"))?;
            println!("wrote {output}: {} pages, {} colors", pages.len(), colors.len());
            Ok(())
        }
        ["font", input, output] => {
            let name = name.ok_or("font needs --name <family>")?;
            let bytes = std::fs::read(input).map_err(|e| format!("{input}: {e}"))?;
            let ext = Path::new(input)
                .extension()
                .map(|e| e.to_string_lossy().to_ascii_lowercase())
                .unwrap_or_default();
            let f = if emb_letter::bitmap::EXTENSIONS.contains(&ext.as_str()) {
                // A bitmap font: the largest face.
                let faces = emb_letter::bitmap::read_file(&ext, &bytes).map_err(|e| format!("{input}: {e}"))?;
                emb_letter::bitmap::to_embfont(&faces[0], &name, stitch)
            } else {
                let font = emb_letter::Font::from_bytes(bytes, 0).map_err(|e| format!("{input}: {e}"))?;
                emb_letter::embfont::convert_with(
                    &font,
                    &name,
                    &style,
                    stitch,
                    &emb_letter::embfont::default_chars(),
                    small_text,
                )
            };
            f.save(Path::new(output))?;
            println!(
                "wrote {output}: {} letters, {} kerning pairs, {:.1}-{:.0} mm",
                f.letters.len(),
                f.kerning.len(),
                f.min_height_mm,
                f.max_height_mm
            );
            Ok(())
        }
        ["quote", input] => {
            let input = read_fabric(input, &out.fabric)?;
            if !order_given
                && let emb_cli::Input::Design(d) = &input
                && let Some(o) = &d.order
            {
                quote.order = o.quote_order();
            }
            let (q, run, s) = emb_cli::quote(&input, machine, &quote)?;
            print!("{}", emb_cli::quote_text(&q, run.as_ref(), &quote.currency));
            if let Some(f) = &sheet {
                let name = match &input {
                    emb_cli::Input::Design(d) => d.name.clone(),
                    emb_cli::Input::Stitches(_) => "design".into(),
                };
                let mut data = emb_render::sheet::SheetData::from_stitches(&name, &s);
                data.date = chrono::Local::now().format("%Y-%m-%d").to_string();
                if let emb_cli::Input::Design(d) = &input {
                    data.notes = d.notes.clone();
                    data.order = d.order.clone();
                    data.background = d.background;
                }
                let page = emb_render::sheet::quote_sheet(&data, &q, &quote.currency, out.paper);
                std::fs::write(f, emb_render::pdf::write(&[page], &format!("{name} quote")))
                    .map_err(|e| format!("{}: {e}", f.display()))?;
                println!("wrote {}", f.display());
            }
            Ok(())
        }
        ["script", script, inputs @ ..] if !inputs.is_empty() => {
            let src = std::fs::read_to_string(script).map_err(|e| format!("{script}: {e}"))?;
            emb_script::check(&src).map_err(|e| format!("{script}: {e}"))?;
            if let Some(d) = &bench_out {
                std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
            }
            let mut failed = 0;
            for input in inputs {
                let path = Path::new(input);
                let name = path
                    .file_stem()
                    .map_or("design".into(), |s| s.to_string_lossy().into_owned());
                let result = read_fabric(input, &out.fabric)
                    .and_then(|i| emb_cli::run_script(&src, i, &name, &script_inputs, machine));
                match result {
                    Ok(o) => {
                        for msg in &o.messages {
                            println!("{input}: {msg}");
                        }
                        match (&bench_out, o.changed) {
                            (Some(dir), true) => {
                                let p = dir.join(format!("{name}.{}", emb_model::project::FILE_EXTENSION));
                                emb_model::project::save(&o.design, &p).map_err(|e| e.to_string())?;
                                println!("{input}: changed, saved to {}", p.display());
                            }
                            (None, true) => println!("{input}: changed (not saved: use --out <dir>)"),
                            (_, false) => println!("{input}: no change"),
                        }
                    }
                    Err(e) => {
                        failed += 1;
                        eprintln!("{input}: the script stopped: {e}");
                    }
                }
            }
            if failed > 0 {
                return Err(format!("the script stopped on {failed} of {} inputs", inputs.len()));
            }
            Ok(())
        }
        ["fabrics"] => {
            print!("{}", emb_cli::fabric_list());
            Ok(())
        }
        ["bench"] | ["bench", _] => {
            let mut results = Vec::new();
            for p in emb_cli::bench::pictures() {
                results.push(emb_cli::bench::run(&p));
            }
            if let [_, dir] = rest.as_slice() {
                for p in emb_cli::bench::pictures_in(Path::new(dir), &bench)? {
                    match p {
                        Ok(p) => results.push(emb_cli::bench::run(&p)),
                        Err(e) => println!("{e}"),
                    }
                }
            }
            print!("{}", emb_cli::bench::report(&results, true));
            if let Some(out) = &bench_out {
                std::fs::create_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
                for m in &results {
                    let path = out.join(format!("{}.stitches.png", m.name));
                    m.stitch_picture
                        .save(&path)
                        .map_err(|e| format!("{}: {e}", path.display()))?;
                }
            }
            Ok(())
        }
        _ => Err(USAGE.to_string()),
    }
}

/// Reads an input and sets its fabric, if one is given.
fn read_fabric(input: &str, fabric: &Option<String>) -> Result<emb_cli::Input, String> {
    let mut i = read_input(Path::new(input))?;
    if let Some(f) = fabric {
        emb_cli::set_fabric(&mut i, f)?;
    }
    Ok(i)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
