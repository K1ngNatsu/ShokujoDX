//! The start of the program (v0.3 plan, Stage 1): the renderer, the
//! fallbacks when the graphics do not start, the start log, and the
//! message when the program cannot start.
//!
//! The program has no console on Windows, so an error at the start went
//! nowhere and the program stopped with no message. Now:
//!
//! - Each start writes `start.log` in the user folder.
//! - If the window or the renderer fails before the first frame (an error
//!   or a panic), the program starts again with the next renderer of
//!   [`Renderer::ORDER`]: the graphics card (DirectX 12, Vulkan or OpenGL),
//!   then OpenGL only, then the software renderer (WARP on Windows).
//! - If all fail, a message box tells the reason and what to do.
//! - A renderer other than the first is kept (`renderer.txt`) with the
//!   list of the graphics adapters, so the next start uses it at once. A
//!   change of the adapters (for example a new driver) makes the program
//!   try the graphics card again.
//! - `--diagnose` writes a report of the graphics adapters.

use std::fmt::Write as _;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

use eframe::egui_wgpu::{self, WgpuConfiguration, WgpuSetup, wgpu};

use crate::i18n::tr;

/// A way to draw the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Renderer {
    /// The graphics card: DirectX 12, Vulkan or OpenGL, as the graphics
    /// library selects.
    Auto,
    /// OpenGL only.
    Gl,
    /// The software renderer of the system (no graphics card).
    Software,
}

impl Renderer {
    /// The renderers in the order that the program tries them.
    pub const ORDER: [Renderer; 3] = [Renderer::Auto, Renderer::Gl, Renderer::Software];

    pub fn name(self) -> &'static str {
        match self {
            Renderer::Auto => "auto",
            Renderer::Gl => "gl",
            Renderer::Software => "software",
        }
    }

    pub fn parse(s: &str) -> Option<Renderer> {
        Self::ORDER
            .into_iter()
            .find(|r| r.name().eq_ignore_ascii_case(s.trim()))
    }

    /// The renderer to try after this one.
    pub fn next(self) -> Option<Renderer> {
        let i = Self::ORDER.iter().position(|r| *r == self)?;
        Self::ORDER.get(i + 1).copied()
    }

    /// The settings of the graphics library for this renderer.
    pub fn wgpu(self) -> WgpuConfiguration {
        let mut config = WgpuConfiguration::default();
        if let WgpuSetup::CreateNew(new) = &mut config.wgpu_setup {
            match self {
                Renderer::Auto => {}
                Renderer::Gl => new.instance_descriptor.backends = wgpu::Backends::GL,
                Renderer::Software => {
                    new.instance_descriptor.backends = wgpu::Backends::all();
                    new.native_adapter_selector = Some(Arc::new(software_adapter));
                }
            }
        }
        config
    }
}

/// Selects a software adapter that can draw on the window.
fn software_adapter(adapters: &[wgpu::Adapter], surface: Option<&wgpu::Surface<'_>>) -> Result<wgpu::Adapter, String> {
    adapters
        .iter()
        .filter(|a| a.get_info().device_type == wgpu::DeviceType::Cpu)
        .find(|a| surface.is_none_or(|s| a.is_surface_supported(s)))
        .cloned()
        .ok_or_else(|| format!("no software adapter: {}", adapter_list(adapters)))
}

/// What this start does: its renderer, and the renderer that failed
/// before it (if this start is a fallback) with the reason.
#[derive(Clone, Debug)]
pub struct Start {
    pub renderer: Renderer,
    pub failed: Option<(Renderer, String)>,
    /// The arguments of the program without the arguments of this module.
    args: Vec<String>,
}

/// The start of this run.
static START: OnceLock<Start> = OnceLock::new();
/// The first frame is drawn: the renderer works.
static STARTED: AtomicBool = AtomicBool::new(false);
/// The argument that selects a renderer.
const RENDERER_ARG: &str = "--renderer=";
/// The argument that tells the renderer that failed before.
const FAILED_ARG: &str = "--failed-renderer=";
/// The environment variable with the reason of the failure.
const REASON_ENV: &str = "SHOKUJODX_START_ERROR";

impl Start {
    /// The start from the arguments (without the program name): an
    /// argument `--renderer=…`, else the kept renderer, else `Auto`.
    pub fn from_args(all: &[String]) -> Start {
        let mut renderer = None;
        let mut failed = None;
        let mut args = Vec::new();
        for a in all {
            if let Some(v) = a.strip_prefix(RENDERER_ARG) {
                renderer = Renderer::parse(v);
            } else if let Some(v) = a.strip_prefix(FAILED_ARG) {
                failed = Renderer::parse(v).map(|r| (r, std::env::var(REASON_ENV).unwrap_or_default()));
            } else {
                args.push(a.clone());
            }
        }
        let renderer = renderer.or_else(kept_renderer).unwrap_or(Renderer::Auto);
        Start { renderer, failed, args }
    }

    /// Makes this start the start of the run and writes it in the log.
    pub fn begin(self) -> &'static Start {
        let fresh = self.failed.is_none();
        let mut text = String::new();
        let _ = writeln!(
            text,
            "{} · {} · renderer {}",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            crate::about::build_text(),
            self.renderer.name()
        );
        if let Some((r, why)) = &self.failed {
            let _ = writeln!(text, "  after renderer {} failed: {why}", r.name());
        }
        log(&text, fresh);
        START.get_or_init(|| self)
    }
}

/// The start of this run, if `Start::begin` was called.
pub fn current() -> Option<&'static Start> {
    START.get()
}

/// The arguments of the program (without the program name and the
/// arguments of this module).
pub fn args() -> Vec<std::ffi::OsString> {
    match current() {
        Some(s) => s.args.iter().map(Into::into).collect(),
        None => std::env::args_os().skip(1).collect(),
    }
}

/// The user folder file `name`.
pub(crate) fn user_file(name: &str) -> Option<PathBuf> {
    crate::about::user_folder().map(|d| d.join(name))
}

/// Writes `text` in `start.log` (a new log if `fresh`).
fn log(text: &str, fresh: bool) {
    let Some(path) = user_file("start.log") else { return };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let old = if fresh {
        String::new()
    } else {
        std::fs::read_to_string(&path).unwrap_or_default()
    };
    let _ = std::fs::write(&path, old + text);
}

/// The kept renderer, if the graphics adapters did not change since it
/// was kept.
fn kept_renderer() -> Option<Renderer> {
    let text = std::fs::read_to_string(user_file("renderer.txt")?).ok()?;
    let (name, adapters) = text.split_once('\n')?;
    let r = Renderer::parse(name)?;
    if adapters.trim_end() == adapter_list(&adapters_now()).trim_end() {
        Some(r)
    } else {
        // A new driver or graphics card: try the graphics card again.
        if let Some(p) = user_file("renderer.txt") {
            let _ = std::fs::remove_file(p);
        }
        None
    }
}

/// The graphics adapters of all the backends.
fn adapters_now() -> Vec<wgpu::Adapter> {
    let mut desc = egui_wgpu::WgpuSetupCreateNew::without_display_handle().instance_descriptor;
    desc.backends = wgpu::Backends::all();
    let instance = wgpu::Instance::new(desc);
    pollster::block_on(instance.enumerate_adapters(wgpu::Backends::all()))
}

/// One line for each adapter: backend, type, name and driver.
fn adapter_list(adapters: &[wgpu::Adapter]) -> String {
    if adapters.is_empty() {
        return "(no adapters)".into();
    }
    let mut lines: Vec<String> = adapters
        .iter()
        .map(|a| {
            let i = a.get_info();
            format!(
                "{:?} · {:?} · {} · {} {}",
                i.backend, i.device_type, i.name, i.driver, i.driver_info
            )
        })
        .collect();
    lines.sort();
    lines.join("\n")
}

/// Call when the first frame is drawn: the renderer works. Keeps a
/// fallback renderer for the next start; the first renderer is not kept.
pub fn first_frame() {
    if STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    let Some(start) = current() else { return };
    log("  first frame drawn\n", false);
    let Some(path) = user_file("renderer.txt") else { return };
    if start.renderer == Renderer::Auto {
        let _ = std::fs::remove_file(path);
    } else {
        let _ = std::fs::write(
            path,
            format!("{}\n{}\n", start.renderer.name(), adapter_list(&adapters_now())),
        );
    }
}

/// `true` if the program draws with the software renderer: after the
/// fallbacks, or when the graphics library chose a processor adapter.
pub fn is_software(adapter: Option<&wgpu::AdapterInfo>) -> bool {
    current().is_some_and(|s| s.renderer == Renderer::Software)
        || adapter.is_some_and(|i| i.device_type == wgpu::DeviceType::Cpu)
}

/// Writes the adapter in use (`adapter`, if known) in the start log, and
/// returns the text that tells the user that the program does not use the
/// graphics card: after a fallback, or when the graphics library selected
/// the software renderer (no graphics driver works).
pub fn graphics_notice(adapter: Option<&wgpu::AdapterInfo>) -> Option<Vec<String>> {
    let start = current()?;
    if let Some(i) = adapter {
        log(
            &format!(
                "  adapter: {:?} · {:?} · {} · {} {}\n",
                i.backend, i.device_type, i.name, i.driver, i.driver_info
            ),
            false,
        );
    }
    let how = if is_software(adapter) {
        tr("ShokujoDX could not use the graphics card, so it draws with the software renderer. It can be slow.")
    } else if start.renderer == Renderer::Gl {
        tr("ShokujoDX draws with OpenGL, because the first way to use the graphics card did not work.")
    } else {
        return None;
    };
    let mut lines = vec![
        how.to_string(),
        tr(
            "Install the newest graphics driver from the maker of your PC or of its graphics (Intel, AMD or NVIDIA), then start ShokujoDX again. It then tries the graphics card again.",
        )
        .to_string(),
    ];
    if let Some((_, why)) = &start.failed
        && !why.is_empty()
    {
        lines.push(crate::i18n::fill("The error was: {}", &[why]));
    }
    Some(lines)
}

/// Call when the renderer or the window failed with `reason`. Before the
/// first frame, the program starts again with the next renderer; if there
/// is none, a message box tells the reason. Returns `true` if a new start
/// runs (this process must stop).
pub fn failed(reason: &str) -> bool {
    let Some(start) = current() else { return false };
    let reason = reason.lines().next().unwrap_or("").trim().to_string();
    log(&format!("  failed: {reason}\n"), false);
    if STARTED.load(Ordering::SeqCst) {
        return false;
    }
    if let Some(next) = start.renderer.next()
        && let Ok(exe) = std::env::current_exe()
    {
        let mut cmd = std::process::Command::new(exe);
        cmd.args(&start.args)
            .arg(format!("{RENDERER_ARG}{}", next.name()))
            .arg(format!("{FAILED_ARG}{}", start.renderer.name()))
            .env(REASON_ENV, &reason);
        if cmd.spawn().is_ok() {
            log(&format!("  starts again with renderer {}\n", next.name()), false);
            return true;
        }
    }
    cannot_start(&reason);
    false
}

/// The message box when no renderer works.
fn cannot_start(reason: &str) {
    let log_path = user_file("start.log")
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let text = format!(
        "{}\n\n{}\n\n{}\n{}",
        tr("ShokujoDX cannot open its window: the graphics did not start."),
        crate::i18n::fill("The error was: {}", &[&reason]),
        tr(
            "Install the newest graphics driver from the maker of your PC or of its graphics (Intel, AMD or NVIDIA), then start ShokujoDX again. If it still does not start, send this file with your feedback:",
        ),
        log_path,
    );
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title("ShokujoDX")
        .set_description(text)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
}

/// Adds to the panic hook: a panic of the main thread (the window and the
/// renderer) before the first frame is a failure of the start (see
/// [`failed`]). A panic of another thread (a background job) is not.
pub fn install_panic_hook() {
    let before = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        before(info);
        if STARTED.load(Ordering::SeqCst) || current().is_none() || !on_main_thread() {
            return;
        }
        let message = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| (*s).to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "(no message)".into());
        let place = info
            .location()
            .map(|l| format!(" ({}:{})", l.file(), l.line()))
            .unwrap_or_default();
        failed(&format!("panic: {message}{place}"));
        std::process::exit(1);
    }));
}

/// Returns `true` on the main thread of the program, which runs the window
/// and the renderer.
fn on_main_thread() -> bool {
    std::thread::current().name() == Some("main")
}

/// `--diagnose`: writes `diagnose.txt` in the user folder (the program,
/// the system, the graphics adapters and the last start log) and shows
/// where it is. Returns the text.
pub fn diagnose() -> String {
    let mut text = String::new();
    let _ = writeln!(text, "ShokujoDX diagnosis");
    let _ = writeln!(text, "Program: {}", crate::about::build_text());
    let _ = writeln!(text, "System: {} {}", std::env::consts::OS, std::env::consts::ARCH);
    let _ = writeln!(text, "Time: {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"));
    let _ = writeln!(
        text,
        "\nGraphics adapters (all backends):\n{}",
        adapter_list(&adapters_now())
    );
    for r in Renderer::ORDER {
        let _ = writeln!(text, "\nRenderer {}: {}", r.name(), probe(r));
    }
    let kept = user_file("renderer.txt").and_then(|p| std::fs::read_to_string(p).ok());
    let _ = writeln!(
        text,
        "\nKept renderer: {}",
        kept.as_deref().and_then(|t| t.lines().next()).unwrap_or("(none)")
    );
    let last = user_file("start.log").and_then(|p| std::fs::read_to_string(p).ok());
    let _ = writeln!(text, "\nLast start log:\n{}", last.as_deref().unwrap_or("(none)"));
    let path = user_file("diagnose.txt");
    let written = path.as_ref().is_some_and(|p| {
        p.parent().is_some_and(|d| std::fs::create_dir_all(d).is_ok()) && std::fs::write(p, &text).is_ok()
    });
    let place = match (&path, written) {
        (Some(p), true) => crate::i18n::fill(
            "The report is in {}. Please send it with your feedback.",
            &[&p.display()],
        ),
        _ => tr("The report could not be written.").to_string(),
    };
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Info)
        .set_title("ShokujoDX")
        .set_description(place)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
    text
}

/// Tries to get an adapter and a device for renderer `r` with no window.
fn probe(r: Renderer) -> String {
    let config = r.wgpu();
    let WgpuSetup::CreateNew(new) = config.wgpu_setup else {
        return "(not tested)".into();
    };
    let backends = new.instance_descriptor.backends;
    let instance = wgpu::Instance::new(new.instance_descriptor);
    let adapters = pollster::block_on(instance.enumerate_adapters(backends));
    let adapter = match &new.native_adapter_selector {
        Some(select) => select(&adapters, None),
        None => pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: new.power_preference,
            compatible_surface: None,
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        }))
        .map_err(|e| e.to_string()),
    };
    let adapter = match adapter {
        Ok(a) => a,
        Err(e) => return format!("no adapter: {e}"),
    };
    let info = adapter.get_info();
    match pollster::block_on(adapter.request_device(&(new.device_descriptor)(&adapter))) {
        Ok(_) => format!("works ({:?}, {:?}, {})", info.backend, info.device_type, info.name),
        Err(e) => format!("no device on {} ({:?}): {e}", info.name, info.backend),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renderers_go_in_order_and_parse() {
        assert_eq!(Renderer::Auto.next(), Some(Renderer::Gl));
        assert_eq!(Renderer::Gl.next(), Some(Renderer::Software));
        assert_eq!(Renderer::Software.next(), None);
        for r in Renderer::ORDER {
            assert_eq!(Renderer::parse(r.name()), Some(r));
        }
        assert_eq!(Renderer::parse("GL"), Some(Renderer::Gl));
        assert_eq!(Renderer::parse("metal"), None);
    }

    #[test]
    fn the_arguments_of_a_fallback_are_taken_out() {
        let args: Vec<String> = ["--renderer=software", "--failed-renderer=gl", "design.embproj"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let s = Start::from_args(&args);
        assert_eq!(s.renderer, Renderer::Software);
        assert_eq!(s.failed.as_ref().map(|f| f.0), Some(Renderer::Gl));
        assert_eq!(s.args, ["design.embproj"]);
    }

    #[test]
    fn only_the_main_thread_fails_the_start() {
        // Test threads have the name of their test.
        assert!(!on_main_thread());
        let worker = std::thread::spawn(on_main_thread).join().unwrap();
        assert!(!worker);
    }

    #[test]
    fn each_renderer_has_a_configuration() {
        let backends = |r: Renderer| match r.wgpu().wgpu_setup {
            WgpuSetup::CreateNew(n) => (n.instance_descriptor.backends, n.native_adapter_selector.is_some()),
            WgpuSetup::Existing(_) => panic!(),
        };
        assert_eq!(backends(Renderer::Gl), (wgpu::Backends::GL, false));
        assert_eq!(backends(Renderer::Software), (wgpu::Backends::all(), true));
    }
}
