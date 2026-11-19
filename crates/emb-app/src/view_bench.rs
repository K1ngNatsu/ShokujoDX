//! `--view-bench` (P18 view plan, Milestone 1, ADR 0122): how fast the
//! canvas of today draws large designs with the renderer of this computer.
//!
//! The program opens a window of its own, not the main window: it reads
//! and writes no settings, makes no backups and opens no design. The
//! window shows synthetic designs of about 10,000, 100,000 and 1,000,000
//! stitches (`emb_render::stress`) with the meshes of the canvas (the
//! stitches, the flat thread look and the 3D thread look), and moves and
//! zooms the view for some seconds for each. The window does not wait for
//! the screen (no vertical sync), so the numbers show the speed of the
//! computer, not the refresh rate of the screen. At the end the program
//! writes `view-bench.txt` in the folder of the user, tells where it is,
//! and stops; `--view-bench=<file>` writes the file given and tells
//! nothing (for scripts). `--renderer=software` measures the software
//! renderer.

use std::fmt::Write as _;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, Shape};
use eframe::egui_wgpu::wgpu;
use emb_render::Segment;

use crate::canvas::{CanvasView, MeshLook, color32, fitted_mesh};
use crate::startup::Renderer;

/// The segments of a design with their colors on the screen.
type Segments = Vec<(Segment, Color32)>;

/// The numbers of stitches of the designs.
const STITCHES: [usize; 3] = [10_000, 100_000, 1_000_000];
/// The time that each case moves the view.
const DURATION: Duration = Duration::from_secs(3);
/// Frames at the start of a case that are not counted.
const WARM_UP: usize = 5;
/// The size of the window, in points.
const WINDOW: [f32; 2] = [1280.0, 720.0];

/// The meshes of the canvas that the bench measures.
const LOOKS: [MeshLook; 3] = [MeshLook::Stitches, MeshLook::Thread, MeshLook::Thread3d];

fn name(look: MeshLook) -> &'static str {
    match look {
        MeshLook::Stitches => "stitches",
        MeshLook::Thread => "thread look",
        MeshLook::Thread3d => "3D thread look",
    }
}

/// The result of one case.
struct Row {
    stitches: usize,
    look: MeshLook,
    /// The lightest look that a frame used, if it is not `look`
    /// (`canvas::fitted_mesh`).
    drawn: Option<MeshLook>,
    frames: usize,
    seconds: f64,
    /// The time between frames, ms, from small to large.
    frame_ms: Vec<f64>,
    mesh_ms: f64,
    triangles: usize,
}

struct Bench {
    renderer: Renderer,
    /// The report file of `--view-bench=<file>`.
    out: Option<PathBuf>,
    cases: Vec<(usize, MeshLook)>,
    case: usize,
    /// The segments of the design of the case: the number of stitches
    /// asked for, the number made, and the segments.
    segments: Option<(usize, usize, Segments)>,
    table: emb_render::thread3d::ShadeTable,
    /// The start of the counted frames of the case, and the frames so far.
    start: Option<Instant>,
    frames: usize,
    last: Option<Instant>,
    frame_ms: Vec<f64>,
    mesh_ms: f64,
    triangles: usize,
    drawn: Option<MeshLook>,
    rows: Vec<Row>,
    adapter: String,
    pixels_per_point: f32,
    size: egui::Vec2,
}

/// The report file of the arguments: `Some(None)` for `--view-bench` (the
/// folder of the user), `Some(Some(file))` for `--view-bench=<file>`,
/// `None` if there is no such argument.
pub fn wanted(args: &[String]) -> Option<Option<PathBuf>> {
    args.iter().find_map(|a| match a.strip_prefix("--view-bench") {
        Some("") => Some(None),
        Some(rest) => rest.strip_prefix('=').map(|f| Some(PathBuf::from(f))),
        None => None,
    })
}

/// Opens the window of the bench with `renderer`, and writes the report
/// when it is done (to `out`, else to the folder of the user).
pub fn run(renderer: Renderer, out: Option<PathBuf>) -> eframe::Result {
    let mut wgpu_options = renderer.wgpu();
    wgpu_options.surface.present_mode = wgpu::PresentMode::AutoNoVsync;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(format!("ShokujoDX {} view bench", crate::about::VERSION))
            .with_inner_size(WINDOW)
            .with_resizable(false),
        renderer: eframe::Renderer::Wgpu,
        wgpu_options,
        ..Default::default()
    };
    let bench = Bench {
        renderer,
        out,
        cases: STITCHES
            .iter()
            .flat_map(|&n| LOOKS.into_iter().map(move |l| (n, l)))
            .collect(),
        case: 0,
        segments: None,
        table: emb_render::thread3d::ShadeTable::new(Default::default()),
        start: None,
        frames: 0,
        last: None,
        frame_ms: Vec::new(),
        mesh_ms: 0.0,
        triangles: 0,
        drawn: None,
        rows: Vec::new(),
        adapter: String::new(),
        pixels_per_point: 1.0,
        size: egui::Vec2::ZERO,
    };
    eframe::run_native("emb-app-view-bench", options, Box::new(|_| Ok(Box::new(bench))))
}

impl eframe::App for Bench {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if self.adapter.is_empty()
            && let Some(rs) = frame.wgpu_render_state()
        {
            let i = rs.adapter.get_info();
            self.adapter = format!("{} ({:?}, {:?}, driver {})", i.name, i.backend, i.device_type, i.driver);
        }
        let Some(&(n, look)) = self.cases.get(self.case) else {
            self.finish();
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        };
        if self.segments.as_ref().is_none_or(|(m, _, _)| *m != n) {
            let design = emb_render::stress::make(n);
            let segs = emb_render::segments(&design)
                .into_iter()
                .map(|s| (s, color32(s.color)))
                .collect();
            self.segments = Some((n, design.stitch_count(), segs));
            self.last = None;
        }
        let now = Instant::now();
        if let Some(last) = self.last {
            self.frames += 1;
            if self.frames == WARM_UP {
                self.start = Some(now);
            } else if self.frames > WARM_UP {
                self.frame_ms.push((now - last).as_secs_f64() * 1000.0);
            }
        }
        self.last = Some(now);
        let rect = ui.max_rect();
        self.size = rect.size();
        self.pixels_per_point = ctx.pixels_per_point();
        let (made, segs) = self.segments.as_ref().map(|s| (s.1, &s.2)).expect("made above");
        // The view moves in a circle and zooms from the whole design to 4
        // times and back.
        let t = self
            .start
            .map_or(0.0, |s| (now - s).as_secs_f64() / DURATION.as_secs_f64());
        let mut view = CanvasView::default();
        let all = emb_render::bounds(&segs.iter().map(|s| s.0).collect::<Vec<_>>()).expect("stitches");
        view.fit(rect, all);
        let (s, c) = emb_geom::math::sin_cos_deg(360.0 * t);
        view.zoom *= 1.0 + 1.5 * (1.0 - c);
        view.center = emb_geom::Point::new(view.center.x + 20.0 * s, view.center.y + 20.0 * (1.0 - c));
        let t0 = Instant::now();
        let (mesh, drawn) = fitted_mesh(rect, &view, segs, false, look, &self.table);
        if self.frames > WARM_UP {
            self.mesh_ms += t0.elapsed().as_secs_f64() * 1000.0;
            self.triangles = self.triangles.max(mesh.indices.len() / 3);
            if drawn != look && self.drawn.is_none_or(|d| d != MeshLook::Stitches) {
                self.drawn = Some(drawn);
            }
        }
        ui.painter().rect_filled(rect, 0.0, Color32::from_rgb(238, 234, 226));
        ui.painter().add(Shape::mesh(mesh));
        if self.start.is_some_and(|s| now - s >= DURATION) {
            self.next_case(made, look, now);
        }
        ctx.request_repaint();
    }
}

impl Bench {
    fn next_case(&mut self, stitches: usize, look: MeshLook, now: Instant) {
        let mut frame_ms = std::mem::take(&mut self.frame_ms);
        frame_ms.sort_by(f64::total_cmp);
        let counted = frame_ms.len();
        self.rows.push(Row {
            stitches,
            look,
            drawn: self.drawn.take(),
            frames: counted,
            seconds: self.start.map_or(0.0, |s| (now - s).as_secs_f64()),
            frame_ms,
            mesh_ms: self.mesh_ms / counted.max(1) as f64,
            triangles: self.triangles,
        });
        self.case += 1;
        (self.frames, self.start, self.last, self.mesh_ms, self.triangles) = (0, None, None, 0.0, 0);
    }

    /// The report: the computer, the renderer and a line for each case.
    fn report(&self) -> String {
        let mut text = String::new();
        let _ = writeln!(text, "ShokujoDX view bench (ADR 0122)");
        let _ = writeln!(text, "Program: {}", crate::about::build_text());
        let _ = writeln!(text, "System: {} {}", std::env::consts::OS, std::env::consts::ARCH);
        let _ = writeln!(text, "Time: {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"));
        let threads = std::thread::available_parallelism().map_or(1, usize::from);
        let _ = writeln!(text, "Processor threads: {threads}");
        let _ = writeln!(text, "Renderer: {}; adapter: {}", self.renderer.name(), self.adapter);
        let _ = writeln!(
            text,
            "Window: {:.0} x {:.0} points, {} pixels for each point; no vertical sync",
            self.size.x, self.size.y, self.pixels_per_point
        );
        let _ = writeln!(
            text,
            "\nEach case moves and zooms the view for {} s (the whole design to 4 times).",
            DURATION.as_secs()
        );
        let _ = writeln!(
            text,
            "{:>9}  {:<15} {:>9} {:>11} {:>11} {:>9} {:>10}  note",
            "stitches", "look", "frames/s", "median ms", "slow ms", "mesh ms", "triangles"
        );
        for r in &self.rows {
            let at = |q: f64| {
                r.frame_ms
                    .get(((r.frame_ms.len() as f64 - 1.0) * q).round() as usize)
                    .copied()
            };
            let note = r.drawn.map_or(String::new(), |d| {
                format!("too large: some frames drawn as {}", name(d))
            });
            let _ = writeln!(
                text,
                "{:>9}  {:<15} {:>9.1} {:>11.2} {:>11.2} {:>9.2} {:>10}  {note}",
                r.stitches,
                name(r.look),
                r.frames as f64 / r.seconds.max(1e-9),
                at(0.5).unwrap_or(0.0),
                at(0.95).unwrap_or(0.0),
                r.mesh_ms,
                r.triangles
            );
        }
        let _ = writeln!(
            text,
            "\nslow ms: 95 % of the frames are faster. mesh ms: the mean time to make the mesh of a frame.\n\
             A mesh of more than {} vertices gets a lighter look (canvas::fitted_mesh).",
            crate::canvas::MAX_MESH_VERTICES
        );
        text
    }

    /// Writes the report, prints it and tells the user where it is.
    fn finish(&mut self) {
        if self.case == usize::MAX {
            return;
        }
        self.case = usize::MAX;
        let text = self.report();
        print!("{text}");
        if let Some(p) = &self.out {
            if let Err(e) = std::fs::write(p, &text) {
                eprintln!("{}: {e}", p.display());
            }
            return;
        }
        let path = crate::startup::user_file("view-bench.txt");
        let written = path.as_ref().is_some_and(|p| {
            p.parent().is_some_and(|d| std::fs::create_dir_all(d).is_ok()) && std::fs::write(p, &text).is_ok()
        });
        let place = match (&path, written) {
            (Some(p), true) => crate::i18n::fill(
                "The report is in {}. Please send it with your feedback.",
                &[&p.display()],
            ),
            _ => crate::i18n::tr("The report could not be written.").to_string(),
        };
        rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Info)
            .set_title("ShokujoDX")
            .set_description(place)
            .set_buttons(rfd::MessageButtons::Ok)
            .show();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_design_size_has_each_look() {
        let cases: Vec<(usize, MeshLook)> = STITCHES
            .iter()
            .flat_map(|&n| LOOKS.into_iter().map(move |l| (n, l)))
            .collect();
        assert_eq!(cases.len(), 9);
        assert_eq!(cases[8], (1_000_000, MeshLook::Thread3d));
        let names: std::collections::BTreeSet<&str> = LOOKS.iter().map(|&l| name(l)).collect();
        assert_eq!(names.len(), 3);
    }

    #[test]
    fn the_argument_gives_the_report_file() {
        let args = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(wanted(&args(&["--lang", "de"])), None);
        assert_eq!(wanted(&args(&["--view-bench"])), Some(None));
        assert_eq!(
            wanted(&args(&["--renderer=software", "--view-bench=out.txt"])),
            Some(Some(PathBuf::from("out.txt")))
        );
        assert_eq!(wanted(&args(&["--view-benchmark"])), None);
    }
}
