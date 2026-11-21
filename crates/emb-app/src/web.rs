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

/// Starts the program on the canvas `shokujodx` of the page.
pub fn start() {
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
        let text = match started {
            Ok(()) => String::new(),
            Err(e) => format!("The program did not start: {e:?}"),
        };
        if let Some(loading) = document.get_element_by_id("loading") {
            loading.set_text_content(Some(&text));
        }
    });
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

/// The first frame: the measures that the address asks for.
fn first_frame(st: &mut AppState) {
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

/// Each frame: the measures, the uploaded files and the notices.
pub fn poll(st: &mut AppState, ctx: &eframe::egui::Context) {
    if !STARTED.swap(true, std::sync::atomic::Ordering::Relaxed) {
        first_frame(st);
    }
    bench_frame(st, ctx);
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
