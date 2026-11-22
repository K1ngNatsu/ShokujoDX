//! The browser trial (roadmap P15, step 1; branch `trial/browser`): the
//! program in a web page. The page starts it on its canvas; files come in
//! by an upload (File > Open) and go out by a download (Save, Export). The
//! commands that need the desktop (the file dialogs, Send to USB, the
//! folders of the user) show a notice. The settings are not kept.

use std::sync::Mutex;

use emb_model::project;
use wasm_bindgen::JsCast;

use crate::app::AppState;
use crate::file::Notice;

/// The notices of commands that are not in the browser.
static NOTICES: Mutex<Vec<String>> = Mutex::new(Vec::new());
/// The files that the user uploaded: their names and bytes.
static UPLOADS: Mutex<Vec<(String, Vec<u8>)>> = Mutex::new(Vec::new());

/// No settings were in the storage of the page when the program started: a
/// first visit (the program saves its settings while it starts).
static FRESH: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Starts the program on the canvas `shokujodx` of the page.
pub fn start() {
    FRESH.store(stored(SETTINGS_KEY).is_none(), std::sync::atomic::Ordering::Relaxed);
    let mut options = eframe::WebOptions::default();
    // `?gl=1`: WebGL2 only, as in a browser with no WebGPU (a measure).
    if param("gl").is_some()
        && let eframe::egui_wgpu::WgpuSetup::CreateNew(new) = &mut options.wgpu_options.wgpu_setup
    {
        new.instance_descriptor.backends = eframe::egui_wgpu::wgpu::Backends::GL;
    }
    wasm_bindgen_futures::spawn_local(async move {
        let Some(document) = web_sys::window().and_then(|w| w.document()) else {
            return;
        };
        let Some(canvas) = document
            .get_element_by_id("shokujodx")
            .and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok())
        else {
            return;
        };
        let started = eframe::WebRunner::new()
            .start(
                canvas,
                options,
                Box::new(|cc| Ok(Box::new(crate::app::EmbApp::new(cc)))),
            )
            .await;
        // The loader of the page (the logo that fills with the download)
        // fades out when the program runs, or shows why it did not start.
        if let Some(loading) = document.get_element_by_id("loading") {
            match started {
                Ok(()) => loading.set_class_name("done"),
                Err(e) => {
                    if let Some(status) = document.get_element_by_id("status") {
                        status.set_text_content(Some(&format!("The program did not start: {e:?}")));
                    }
                }
            }
        }
    });
}

/// The key of the settings in the storage of the page.
pub const SETTINGS_KEY: &str = "shokujodx-settings";

/// The storage of the page (it stays on this computer, in this browser).
fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

/// The text under `key` in the storage of the page.
pub fn stored(key: &str) -> Option<String> {
    storage()?.get_item(key).ok()?
}

/// Keeps `text` under `key` in the storage of the page.
pub fn store(key: &str, text: &str) -> Result<(), String> {
    storage()
        .ok_or("the browser keeps no data for this page")?
        .set_item(key, text)
        .map_err(|e| format!("{e:?}"))
}

/// A command that needs the desktop: the next frame shows a notice.
pub fn not_in_the_browser() {
    if let Ok(mut n) = NOTICES.lock() {
        n.push(
            crate::i18n::tr(
                "This command needs the desktop program. In the browser, File > Open uploads a file, and Save and Export download one.",
            )
            .to_string(),
        );
    }
}

/// Asks the browser for a file to open. The file comes in a later frame.
pub fn upload() {
    wasm_bindgen_futures::spawn_local(async {
        let mut all = vec![project::FILE_EXTENSION];
        all.extend(emb_formats::READ_EXTENSIONS);
        all.extend(["svg", "pdf", "ai", "eps", "png", "jpg", "jpeg", "bmp"]);
        let Some(file) = rfd::AsyncFileDialog::new()
            .add_filter("Designs, machine files, artwork and pictures", &all)
            .pick_file()
            .await
        else {
            return;
        };
        let bytes = file.read().await;
        if let Ok(mut u) = UPLOADS.lock() {
            u.push((file.file_name(), bytes));
        }
    });
}

/// Gives `bytes` to the browser as a download with the name `name`.
pub fn download(name: &str, bytes: &[u8]) -> Result<(), String> {
    let fail = |e: wasm_bindgen::JsValue| format!("{e:?}");
    let document = web_sys::window().and_then(|w| w.document()).ok_or("no page")?;
    let parts = js_sys::Array::new();
    parts.push(&js_sys::Uint8Array::from(bytes));
    let options = web_sys::BlobPropertyBag::new();
    options.set_type("application/octet-stream");
    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &options).map_err(fail)?;
    let url = web_sys::Url::create_object_url_with_blob(&blob).map_err(fail)?;
    let link = document
        .create_element("a")
        .map_err(fail)?
        .dyn_into::<web_sys::HtmlAnchorElement>()
        .map_err(|_| "no link".to_string())?;
    link.set_href(&url);
    link.set_download(name);
    link.click();
    let _ = web_sys::Url::revoke_object_url(&url);
    Ok(())
}

/// The measures of the trial: `?stress=<stitches>` opens a synthetic design
/// with about that many stitches (`emb_render::stress`), `look=` sets the
/// thread look (`flat`, `fast`, `good`, `best`), and `bench=` draws frames
/// with no pause (`still`, `pan` or `zoom`) and writes the frame times to
/// `window.shokujodxStats` (JSON) for the test page.
struct Bench {
    mode: String,
    last: Option<crate::clock::Instant>,
    frames: u32,
    total_ms: f64,
    slowest_ms: f64,
    load_ms: f64,
}

static BENCH: Mutex<Option<Bench>> = Mutex::new(None);
static STARTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// The value of `key` in the query of the address of the page.
fn param(key: &str) -> Option<String> {
    let search = web_sys::window()?.location().search().ok()?;
    search
        .trim_start_matches('?')
        .split('&')
        .filter_map(|kv| kv.split_once('='))
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v.to_string())
}

/// Fetches `url` (a file next to the page) as if the user uploaded it.
fn fetch_as_upload(url: String) {
    wasm_bindgen_futures::spawn_local(async move {
        let Some(window) = web_sys::window() else { return };
        let Ok(response) = wasm_bindgen_futures::JsFuture::from(window.fetch_with_str(&url)).await else {
            return;
        };
        let Ok(response) = response.dyn_into::<web_sys::Response>() else {
            return;
        };
        let Ok(buffer) = response.array_buffer() else { return };
        let Ok(buffer) = wasm_bindgen_futures::JsFuture::from(buffer).await else {
            return;
        };
        let bytes = js_sys::Uint8Array::new(&buffer).to_vec();
        let name = url.rsplit('/').next().unwrap_or("design").to_string();
        if let Ok(mut u) = UPLOADS.lock() {
            u.push((name, bytes));
        }
    });
}

/// After an upload of the measure (`?open=`): the export that `?export=`
/// asks for (a download).
static EXPORT_AFTER_OPEN: Mutex<Option<String>> = Mutex::new(None);

/// `true` if the main pointer of the device is a finger (a phone or a
/// tablet), not a mouse.
fn touch_device() -> bool {
    web_sys::window()
        .and_then(|w| w.match_media("(pointer: coarse)").ok().flatten())
        .is_some_and(|m| m.matches())
}

/// The width of the page in CSS pixels.
fn page_width() -> f64 {
    web_sys::window()
        .and_then(|w| w.inner_width().ok())
        .and_then(|v| v.as_f64())
        .unwrap_or(1024.0)
}

/// The first visit on a touch screen (P15 step 3): larger controls on a
/// tablet (text size 125 %), simple mode on a phone (large buttons, fewer
/// tools). Only with no saved settings, so a choice of the user stays.
fn touch_defaults(st: &mut AppState) {
    if !FRESH.load(std::sync::atomic::Ordering::Relaxed) || !touch_device() {
        return;
    }
    let mut s = st.settings.clone();
    if page_width() < 700.0 {
        s.simple_mode = true;
    } else {
        s.text_scale = 1.25;
    }
    st.set_settings(s);
}

/// The first frame: what the trial is, the defaults of a touch screen, and
/// the measures that the address asks for.
fn first_frame(st: &mut AppState) {
    touch_defaults(st);
    st.message = crate::i18n::tr(
        "This is a trial of ShokujoDX in a browser. File > Open uploads a design or a machine file, and Save and Export download a file. Your designs stay on your computer.",
    )
    .to_string();
    let mut load_ms = 0.0;
    if let Some(url) = param("open") {
        st.ask_machine = None;
        st.show_start = false;
        st.focus_canvas = true;
        if let Ok(mut e) = EXPORT_AFTER_OPEN.lock() {
            *e = param("export");
        }
        fetch_as_upload(url);
    }
    if let Some(text) = param("letter") {
        let text = js_sys::decode_uri_component(&text)
            .ok()
            .and_then(|t| t.as_string())
            .unwrap_or(text);
        st.ask_machine = None;
        st.show_start = false;
        st.focus_canvas = true;
        let t = crate::clock::Instant::now();
        let dialog = crate::lettering::LetteringDialog::new(
            emb_geom::Point::new(-60.0, 0.0),
            &text,
            emb_model::FontRef::new("Lato", "Bold"),
            20.0,
        );
        st.finish_lettering(dialog, true);
        load_ms = t.elapsed().as_secs_f64() * 1000.0;
    }
    if let Some(n) = param("stress").and_then(|v| v.parse::<usize>().ok()) {
        let t = crate::clock::Instant::now();
        let s = emb_render::stress::make(n.min(5_000_000));
        let d = emb_model::design_from_stitches(&s, "Stress");
        st.start_design(d);
        load_ms = t.elapsed().as_secs_f64() * 1000.0;
        st.ask_machine = None;
        st.show_start = false;
        st.focus_canvas = true;
        let ids: Vec<_> = st.doc.design().objects().iter().map(|o| o.id).collect();
        if let Some(b) = st.doc.design().bbox_of(&ids) {
            st.zoom_to = Some(b);
        }
    }
    if let Some(look) = param("look") {
        st.show_thread_look = true;
        let l = &mut st.settings.look;
        l.measured = true;
        match look.as_str() {
            "flat" => l.three_d = false,
            "fast" => {
                l.three_d = true;
                l.set_quality(crate::viewtools::Quality::Fast);
            }
            "good" => {
                l.three_d = true;
                l.set_quality(crate::viewtools::Quality::Good);
            }
            _ => {
                l.three_d = true;
                l.set_quality(crate::viewtools::Quality::Best);
            }
        }
    }
    if let Some(mode) = param("bench") {
        st.ask_machine = None;
        st.show_start = false;
        st.focus_canvas = true;
        if let Ok(mut b) = BENCH.lock() {
            *b = Some(Bench {
                mode,
                last: None,
                frames: 0,
                total_ms: 0.0,
                slowest_ms: 0.0,
                load_ms,
            });
        }
    }
}

/// A frame of the measure: the time since the frame before, a small pan or
/// zoom, and the next frame at once.
fn bench_frame(st: &mut AppState, ctx: &eframe::egui::Context) {
    let Ok(mut guard) = BENCH.lock() else { return };
    let Some(b) = guard.as_mut() else { return };
    let now = crate::clock::Instant::now();
    if let Some(last) = b.last {
        let ms = now.duration_since(last).as_secs_f64() * 1000.0;
        b.frames += 1;
        b.total_ms += ms;
        b.slowest_ms = b.slowest_ms.max(ms);
    }
    b.last = Some(now);
    let k = f64::from(b.frames % 120);
    match b.mode.as_str() {
        "pan" => st.view.center.x += if k < 60.0 { 0.5 } else { -0.5 } / st.view.zoom.max(0.01),
        "zoom" => st.view.zoom *= if k < 60.0 { 1.01 } else { 1.0 / 1.01 },
        _ => {}
    }
    if b.frames % 30 == 0 && b.frames > 0 {
        let stats = format!(
            "{{\"mode\":\"{}\",\"frames\":{},\"mean_ms\":{:.2},\"slowest_ms\":{:.2},\"load_ms\":{:.1}}}",
            b.mode,
            b.frames,
            b.total_ms / f64::from(b.frames),
            b.slowest_ms,
            b.load_ms
        );
        if let Some(w) = web_sys::window() {
            let _ = js_sys::Reflect::set(&w, &"shokujodxStats".into(), &stats.into());
        }
    }
    ctx.request_repaint();
}

/// `?probe=1`: the state of the view for a test of the page
/// (`window.shokujodxView`, JSON), each frame.
static PROBE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

fn probe(st: &AppState, ctx: &eframe::egui::Context) {
    if !*PROBE.get_or_init(|| param("probe").is_some()) {
        return;
    }
    let (touching, middle) = ctx.input(|i| (i.any_touches(), i.multi_touch().map(|t| t.center_pos)));
    let middle = middle.map_or("null".to_string(), |p| format!("[{:.0},{:.0}]", p.x, p.y));
    let canvas = st.canvas_rect.map_or("null".to_string(), |r| {
        format!("[{:.0},{:.0},{:.0},{:.0}]", r.min.x, r.min.y, r.max.x, r.max.y)
    });
    let state = format!(
        "{{\"zoom\":{:.4},\"cx\":{:.3},\"cy\":{:.3},\"gesture\":{},\"simple\":{},\"text_scale\":{},\"undo\":{},\"touching\":{touching},\"middle\":{middle},\"canvas\":{canvas},\"egui\":[{:.2},{:.2},{:.0},{:.0}]}}",
        st.view.zoom,
        st.view.center.x,
        st.view.center.y,
        st.touch_gesture,
        st.settings.simple_mode,
        st.settings.text_scale,
        st.doc.undo_len(),
        ctx.zoom_factor(),
        ctx.pixels_per_point(),
        ctx.content_rect().width(),
        ctx.content_rect().height()
    );
    if let Some(w) = web_sys::window() {
        let _ = js_sys::Reflect::set(&w, &"shokujodxView".into(), &state.into());
    }
}

/// The build of a page: its version (a hash of the program), the time of
/// the build and the commit (`tools/build-web.py` writes them).
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize)]
#[serde(default)]
pub struct Build {
    pub version: String,
    pub built: String,
    pub commit: String,
}

/// The state of File > Check for Updates.
#[derive(Clone, Debug, PartialEq)]
pub enum PageCheck {
    Checking,
    /// The page is the newest build.
    Newest,
    /// A newer build is on the server.
    Newer(Build),
    Failed(String),
}

static PAGE_CHECK: Mutex<Option<PageCheck>> = Mutex::new(None);

/// The build of this page (`window.shokujodxBuild`).
fn this_build() -> Build {
    let field = |o: &wasm_bindgen::JsValue, k: &str| {
        js_sys::Reflect::get(o, &k.into())
            .ok()
            .and_then(|v| v.as_string())
            .unwrap_or_default()
    };
    web_sys::window()
        .and_then(|w| js_sys::Reflect::get(&w, &"shokujodxBuild".into()).ok())
        .filter(|o| o.is_object())
        .map(|o| Build {
            version: field(&o, "version"),
            built: field(&o, "built"),
            commit: field(&o, "commit"),
        })
        .unwrap_or_default()
}

/// The result of the check: the newest build (the text of version.json)
/// against the build of this page.
pub fn compare(this: &Build, newest: &str) -> PageCheck {
    match serde_json::from_str::<Build>(newest) {
        Ok(n) if n.version.is_empty() => PageCheck::Failed("version.json has no version".into()),
        Ok(n) if n.version == this.version => PageCheck::Newest,
        Ok(n) => PageCheck::Newer(n),
        Err(e) => PageCheck::Failed(e.to_string()),
    }
}

/// Calls the function `name` of the page (no arguments) and waits for its
/// promise: the text that it gives.
async fn call_page(name: &str) -> Result<String, String> {
    let w = web_sys::window().ok_or("no window")?;
    let f: js_sys::Function = js_sys::Reflect::get(&w, &name.into())
        .ok()
        .and_then(|f| f.dyn_into().ok())
        .ok_or("the page is older than the program")?;
    let promise: js_sys::Promise = f
        .call0(&w)
        .map_err(|e| format!("{e:?}"))?
        .dyn_into()
        .map_err(|_| "no promise".to_string())?;
    let value = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(|e| e.as_string().unwrap_or_else(|| format!("{e:?}")))?;
    Ok(value.as_string().unwrap_or_default())
}

/// File > Check for Updates in the browser: asks the server for the newest
/// build; the window shows the result.
pub fn check_for_update() {
    if let Ok(mut c) = PAGE_CHECK.lock() {
        *c = Some(PageCheck::Checking);
    }
    wasm_bindgen_futures::spawn_local(async {
        let result = match call_page("shokujodxNewest").await {
            Ok(text) => compare(&this_build(), &text),
            Err(e) => PageCheck::Failed(e),
        };
        if let Ok(mut c) = PAGE_CHECK.lock() {
            // The user can close the window before the answer comes.
            if c.is_some() {
                *c = Some(result);
            }
        }
    });
}

/// The window of File > Check for Updates.
fn update_window(ctx: &eframe::egui::Context) {
    use eframe::egui;
    let Some(state) = PAGE_CHECK.lock().ok().and_then(|c| c.clone()) else {
        return;
    };
    let this = this_build();
    let mut open = true;
    let mut close = false;
    let width = (ctx.content_rect().width() - 32.0).clamp(200.0, 420.0);
    egui::Window::new(crate::i18n::tr("Check for Updates"))
        .id(egui::Id::new("page_update"))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .max_width(width)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            ui.set_max_width(width);
            ui.label(crate::i18n::fill(
                "This page: built {} (version {}).",
                &[&this.built, &this.commit],
            ));
            ui.add_space(6.0);
            match &state {
                PageCheck::Checking => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(crate::i18n::tr("Looking for a newer version…"));
                    });
                }
                PageCheck::Newest => {
                    ui.label(crate::i18n::tr("This is the newest version."));
                }
                PageCheck::Newer(b) => {
                    ui.label(crate::i18n::fill(
                        "A newer version is ready: built {} (version {}).",
                        &[&b.built, &b.commit],
                    ));
                    ui.label(crate::i18n::tr(
                        "A reload closes the open designs. Save them first: File > Save downloads them.",
                    ));
                    ui.add_space(4.0);
                    if ui.button(crate::i18n::tr("Reload Now")).clicked() {
                        wasm_bindgen_futures::spawn_local(async {
                            let _ = call_page("shokujodxUpdate").await;
                        });
                    }
                }
                PageCheck::Failed(e) => {
                    ui.label(crate::i18n::fill(
                        "The check did not work ({}). Is the device online?",
                        &[e],
                    ));
                }
            }
            ui.add_space(4.0);
            if ui.button(crate::i18n::tr("Close")).clicked() {
                close = true;
            }
        });
    if (!open || close)
        && let Ok(mut c) = PAGE_CHECK.lock()
    {
        *c = None;
    }
    if state == PageCheck::Checking {
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
}

/// Each frame: the measures, the uploaded files and the notices.
pub fn poll(st: &mut AppState, ctx: &eframe::egui::Context) {
    update_window(ctx);
    if !STARTED.swap(true, std::sync::atomic::Ordering::Relaxed) {
        first_frame(st);
    }
    bench_frame(st, ctx);
    probe(st, ctx);
    let uploads = UPLOADS.lock().map(|mut u| std::mem::take(&mut *u)).unwrap_or_default();
    for (name, bytes) in uploads {
        st.open_upload(&name, bytes);
        let export = EXPORT_AFTER_OPEN.lock().ok().and_then(|mut e| e.take());
        if let Some(ext) = export {
            let format = crate::export::Format::ALL
                .into_iter()
                .find(|f| f.ext().eq_ignore_ascii_case(&ext));
            if let Some(f) = format
                && let Err(e) = st.download_machine_file(f)
            {
                st.message = e;
            }
        }
    }
    if let Some(n) = NOTICES.lock().ok().and_then(|mut n| n.pop()) {
        st.message = n;
    }
}

impl AppState {
    /// Opens a file that the user uploaded: a project or a machine file as
    /// a new design, artwork into the design, a picture as the backdrop.
    pub fn open_upload(&mut self, name: &str, bytes: Vec<u8>) {
        let path = std::path::Path::new(name);
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Design".into());
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase)
            .unwrap_or_default();
        if ext == project::FILE_EXTENSION {
            match project::from_str(&String::from_utf8_lossy(&bytes)) {
                Ok(d) => {
                    self.start_design(d);
                    self.message = crate::i18n::fill("Opened {}", &[&name]);
                }
                Err(e) => {
                    self.notice = Some(Notice {
                        title: "Cannot open design".into(),
                        lines: vec![name.to_string(), e.to_string()],
                    })
                }
            }
        } else if let Some(ext) = crate::file::machine_ext(path) {
            match emb_formats::read_machine(&ext, &bytes) {
                Ok(s) if s.stitch_count() > 0 => {
                    let mut d = emb_model::design_from_stitches(&s, &stem);
                    d.pin_block_needles(&emb_formats::read_needles(&ext, &bytes));
                    d.machine = self.doc.design().machine.clone();
                    d.hoop.selection = self.doc.design().hoop.selection.clone();
                    let n = d.objects().len();
                    self.start_design(d);
                    self.message = crate::i18n::fill(
                        "Opened {} as {} stitch objects. Save it to keep it as a project.",
                        &[&name, &n],
                    );
                }
                Ok(_) => self.message = crate::i18n::tr("The file has no stitches.").to_string(),
                Err(e) => {
                    self.notice = Some(Notice {
                        title: "Cannot open machine file".into(),
                        lines: vec![name.to_string(), e],
                    })
                }
            }
        } else if crate::backdrop::FILE_TYPES.contains(&ext.as_str()) {
            self.import_bitmap_bytes(bytes, name);
        } else {
            self.import_svg_bytes(&bytes, name);
        }
    }

    /// Save in the browser: the project file as a download.
    pub fn download_project(&mut self) -> bool {
        let name = format!("{}.{}", self.doc.design().name, project::FILE_EXTENSION);
        let saved = project::checked_string(self.doc.design())
            .map_err(|e| e.to_string())
            .and_then(|text| download(&name, text.as_bytes()));
        match saved {
            Ok(()) => {
                self.doc.mark_saved();
                self.message = crate::i18n::fill("Saved {}", &[&name]);
                true
            }
            Err(e) => {
                self.notice = Some(Notice {
                    title: "Cannot save design".into(),
                    lines: vec![name, e],
                });
                false
            }
        }
    }

    /// Export in the browser: the machine file as a download.
    pub fn download_machine_file(&mut self, format: crate::export::Format) -> Result<(), String> {
        if let Some(why) = format.not_ready() {
            return Err(why.to_string());
        }
        let plan = self.plan_view().plan.clone();
        let name = self.doc.design().name.clone();
        let date = crate::output::local_stamp();
        let machine = self.machine_for_files(self.doc.design().machine.as_deref());
        let needles = self.catalogs().needle_plan(self.doc.design(), &plan.stitches);
        let bytes = emb_formats::write_machine_with(
            format.ext(),
            &plan.stitches,
            &name,
            &date,
            machine.as_ref(),
            needles.as_ref(),
        )
        .ok_or_else(|| crate::i18n::fill("{} cannot be written", &[&format.label()]))?;
        let file = format!("{name}.{}", format.ext());
        download(&file, &bytes)?;
        self.message = crate::i18n::fill("Exported {}", &[&file]);
        Ok(())
    }
}
