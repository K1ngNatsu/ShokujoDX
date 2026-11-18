//! ShokujoDX: the desktop program (egui). The engine crates make
//! the stitches and the files; this crate has the ribbon, the canvas, the
//! panels and the windows.
#![forbid(unsafe_code)]
// Hide the console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod a11y;
mod about;
mod actions;
mod alphabets;
mod app;
mod applique;
mod artwork;
mod assets;
mod backdrop;
mod backup;
mod bling;
mod buttonholes;
mod canvas;
mod cleanup;
mod clipart;
mod color_fills;
mod commands;
mod corners;
mod crash;
mod cross;
mod cutwork;
mod decor;
mod density;
mod dock;
mod draw;
mod drawing_aids;
mod editing;
mod effects_ui;
mod export;
mod file;
mod first_start;
mod fonts;
mod functions;
mod gaps;
mod guides;
mod help;
mod hooping;
mod hoops;
mod i18n;
mod icons;
mod info;
mod jobs;
mod join;
mod keyboard;
mod kiosk;
mod letter_edit;
mod lettering;
mod library;
mod manipulate;
mod menus;
mod monogram;
mod motifs;
mod my_machines;
mod needles;
mod order;
mod output;
mod overview;
#[cfg(test)]
mod packaging;
mod palette;
mod panels;
mod picture;
mod player;
mod preview;
mod product;
mod product_photo;
#[cfg(test)]
mod product_probe;
mod quilting;
mod quote;
mod repeats;
mod reshape;
mod resources;
mod ribbon;
mod scripts;
mod selection;
mod send;
mod sequence_panel;
mod setup;
mod share;
mod simplify;
mod sketch;
#[cfg(test)]
mod smoke;
mod start;
mod startup;
mod stitch_types;
mod stock;
#[cfg(test)]
mod stress;
mod styles;
mod tabs;
mod teamnames;
#[cfg(test)]
mod test_util;
mod threads;
mod turntable;
mod vectors;
mod viewing;
mod views;
mod viewtools;
mod wizard;
mod words;
mod workspace;

use eframe::egui;

/// The text of `--help`.
const USAGE: &str = "Use: shokujodx [--lang <code>] [--kiosk | <file>]

  <file>         open a design, a machine file, artwork or a picture
  --lang <code>  the language for this run: en, de, es or ja
  --kiosk        start in the lettering kiosk
  --thumbnail <file> <out.png> [<size>]
                 write the picture of a design file (for the file manager)
  --version      write the version and stop
  --diagnose     write a report of the graphics adapters (diagnose.txt in
                 the settings folder) and stop
  --renderer=<auto|gl|software>
                 draw with this renderer (the program selects one if this
                 is not given, and tries the next if one fails)
  --help         write this text and stop";

/// `--thumbnail <file> <out.png> [<size>]`: the picture of a design file, at
/// most `size` pixels (256 if not given) on its longest side (P11 Milestone 3).
fn thumbnail(args: &[String]) -> Result<(), String> {
    let [input, output, rest @ ..] = args else {
        return Err(USAGE.into());
    };
    let size: u32 = match rest.first() {
        Some(s) => s.parse().map_err(|_| format!("size {s:?} is not a number"))?,
        None => 256,
    };
    let path = std::path::Path::new(input);
    let bytes = std::fs::read(path).map_err(|e| format!("{input}: {e}"))?;
    let ext = path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase());
    let picture = emb_library::preview::picture(&bytes, ext.as_deref(), size)
        .ok_or_else(|| format!("{input}: not a design with stitches"))?;
    std::fs::write(output, picture.png()).map_err(|e| format!("{output}: {e}"))
}

fn main() -> eframe::Result {
    // The packages run `--version` to see that the program starts.
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--version") {
        println!("{}", about::build_text());
        return Ok(());
    }
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{USAGE}");
        return Ok(());
    }
    // The thumbnailers of Linux and macOS run the program for a picture.
    if args.first().map(String::as_str) == Some("--thumbnail") {
        if let Err(e) = thumbnail(&args[1..]) {
            eprintln!("{e}");
            std::process::exit(1);
        }
        return Ok(());
    }
    // The messages before the window use the language of the settings.
    let settings = workspace::Settings::load();
    match args.iter().position(|a| a == "--lang").and_then(|i| args.get(i + 1)) {
        Some(code) => i18n::set_language(code),
        None => i18n::set_language(&settings.language),
    }
    if args.iter().any(|a| a == "--diagnose") {
        startup::diagnose();
        return Ok(());
    }
    // The renderer, and the start again with the next renderer if this one
    // fails (v0.3 plan, Stage 1).
    let start = startup::Start::from_args(&args).begin();
    // A test build writes a report when it stops because of an error.
    crash::install();
    startup::install_panic_hook();
    let mut viewport = egui::ViewportBuilder::default()
        .with_title(format!("ShokujoDX {}", about::VERSION))
        .with_app_id(icons::APP_ID)
        .with_inner_size([1280.0, 800.0])
        .with_min_inner_size([800.0, 500.0]);
    if let Some(icon) = icons::window_icon() {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Wgpu,
        wgpu_options: start.renderer.wgpu(),
        ..Default::default()
    };
    let result = eframe::run_native("emb-app", options, Box::new(|cc| Ok(Box::new(app::EmbApp::new(cc)))));
    match result {
        // The next renderer runs in a new process.
        Err(e) if startup::failed(&e.to_string()) => Ok(()),
        other => other,
    }
}
