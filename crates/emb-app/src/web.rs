//! The browser trial (roadmap P15, step 1; branch `trial/browser`): the
//! program in a web page. The page starts it on its canvas; files come in
//! by an upload (File > Open; a picture for Picture to Design, Load Picture
//! and the Backdrop panel; the commands that ask for a file through
//! `dialog::open_then`) and go out by a download (Save, Export, the commands
//! that save a file; a folder is one zip file, see `crate::dialog`). The
//! other commands that need the desktop (Send to USB, the folders of the
//! user: the library, fonts and assets) show a notice.

use std::sync::Mutex;

use emb_model::project;
use wasm_bindgen::JsCast;

use crate::app::AppState;
use crate::dock::FitScreen;
use crate::file::Notice;

/// The notices of commands that are not in the browser.
static NOTICES: Mutex<Vec<String>> = Mutex::new(Vec::new());
/// The files that the user uploaded: their names, bytes and what to do
/// with them.
static UPLOADS: Mutex<Vec<(String, Vec<u8>, Then)>> = Mutex::new(Vec::new());

/// The context of the program: an upload wakes it, as the program draws a
/// frame only after an input (with no move of the pointer, the file waited
/// for one).
static CONTEXT: std::sync::OnceLock<eframe::egui::Context> = std::sync::OnceLock::new();

/// Keeps the file `name` with its `bytes` for the next frame, and asks for
/// that frame.
fn uploaded(name: String, bytes: Vec<u8>, then: Then) {
    if let Ok(mut u) = UPLOADS.lock() {
        u.push((name, bytes, then));
    }
    if let Some(ctx) = CONTEXT.get() {
        ctx.request_repaint();
    }
}

/// What the program does with an uploaded file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Then {
    /// File > Open: a design, a machine file, artwork or a picture.
    Open,
    /// A picture as the backdrop (Load Picture, Replace).
    Backdrop,
    /// A picture as the backdrop, then the Picture to Design wizard on it.
    Wizard,
    /// The command that asked for it goes on ([`upload_then`]).
    Waiting,
}

/// A command that waits for an upload: it gets the path of the file
/// (`dialog::open_then`). One at a time: a new one takes the place of the
/// one before.
type Waiting = Box<dyn FnOnce(&mut AppState, std::path::PathBuf)>;

thread_local! {
    static WAITING: std::cell::RefCell<Option<Waiting>> = const { std::cell::RefCell::new(None) };
    /// The last file of a waiting command: its path and bytes
    /// (`dialog::read`).
    static FILE: std::cell::RefCell<Option<(std::path::PathBuf, Vec<u8>)>> = const { std::cell::RefCell::new(None) };
    /// The files of the folder of `dialog::pick_folder` in this frame: one
    /// zip file at the end of the frame.
    static ZIP: std::cell::RefCell<Vec<(String, Vec<u8>)>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Asks the browser for a file of `filters` (names and extensions); `then`
/// gets its path in a later frame (see `dialog::open_then`).
pub fn upload_then(filters: Vec<(String, Vec<String>)>, then: Waiting) {
    WAITING.with(|w| *w.borrow_mut() = Some(then));
    wasm_bindgen_futures::spawn_local(async move {
        let mut dialog = rfd::AsyncFileDialog::new();
        for (name, extensions) in &filters {
            dialog = dialog.add_filter(name, extensions);
        }
        let Some(file) = dialog.pick_file().await else {
            return;
        };
        let bytes = file.read().await;
        uploaded(file.file_name(), bytes, Then::Waiting);
    });
}

/// The bytes of the uploaded file at `path` (see `dialog::read`).
pub fn uploaded_file(path: &std::path::Path) -> Option<Vec<u8>> {
    FILE.with(|f| f.borrow().as_ref().filter(|(p, _)| p == path).map(|(_, b)| b.clone()))
}

/// A file to save: a download, or a file of the zip file of a folder
/// (`dialog::write`).
pub fn write_download(path: &std::path::Path, bytes: &[u8]) -> Result<(), String> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .ok_or_else(|| format!("{}: no file name", path.display()))?;
    if path.parent() == Some(std::path::Path::new(crate::dialog::ZIP_FOLDER)) {
        ZIP.with(|z| z.borrow_mut().push((name, bytes.to_vec())));
        if let Some(ctx) = CONTEXT.get() {
            ctx.request_repaint();
        }
        return Ok(());
    }
    download(&name, bytes)
}

/// The zip file of a folder, as one download (see [`write_download`]).
fn download_zip() {
    let files = ZIP.with(|z| std::mem::take(&mut *z.borrow_mut()));
    if !files.is_empty()
        && let Err(e) = download(crate::dialog::ZIP_FOLDER, &crate::share::zip(&files))
        && let Ok(mut n) = NOTICES.lock()
    {
        n.push(e);
    }
}

/// No settings were in the storage of the page when the program started: a
/// first visit (the program saves its settings while it starts).
static FRESH: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Starts the program on the canvas `shokujodx` of the page.
pub fn start() {
    FRESH.store(stored(SETTINGS_KEY).is_none(), std::sync::atomic::Ordering::Relaxed);
    show_panics();
    listen_for_pen();
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
        // goes when the program runs (`window.shokujodxStarted` lets the
        // needle and the star light up first), or shows why it did not
        // start.
        if let Some(loading) = document.get_element_by_id("loading") {
            match started {
                Ok(()) => match page_function("shokujodxStarted") {
                    Some((w, f)) => {
                        let _ = f.call0(&w);
                    }
                    None => loading.set_class_name("done"),
                },
                Err(e) => {
                    if let Some(status) = document.get_element_by_id("status") {
                        status.set_text_content(Some(&format!("The program did not start: {e:?}")));
                    }
                }
            }
        }
    });
}

/// An error that stops the program shows on the page, with its text: the
/// page stopped with no word before (the owner, 2026-11-25: "the site
/// froze"). eframe puts its hook over this one and calls it.
fn show_panics() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        previous(info);
        let Some(document) = web_sys::window().and_then(|w| w.document()) else {
            return;
        };
        let Ok(note) = document.create_element("div") else {
            return;
        };
        let _ = note.set_attribute(
            "style",
            "position:fixed;left:0;right:0;top:0;z-index:30;padding:12px 16px;             background:#b00020;color:#fff;font:15px system-ui,sans-serif;cursor:pointer",
        );
        let _ = note.set_attribute("onclick", "location.reload()");
        note.set_text_content(Some(&format!(
            "The program stopped because of an error. Tap here to reload the page.              Please send this text to the developer: {info}"
        )));
        if let Some(body) = document.body() {
            let _ = body.append_child(&note);
        }
    }));
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
    let mut all = vec![project::FILE_EXTENSION];
    all.extend(emb_formats::READ_EXTENSIONS);
    all.extend(["svg", "pdf", "ai", "eps", "png", "jpg", "jpeg", "bmp"]);
    ask_for_file("Designs, machine files, artwork and pictures".into(), all, Then::Open);
}

/// Asks the browser for a picture: it becomes the backdrop and, with
/// `wizard`, the Picture to Design wizard opens on it. The desktop asks
/// with the dialog of the system; in the browser that dialog is not there,
/// and the command only said so (the owner, 2026-11-23).
pub fn upload_picture(wizard: bool) {
    let then = if wizard { Then::Wizard } else { Then::Backdrop };
    ask_for_file(
        crate::i18n::tr("Pictures (PNG, JPEG, BMP, TIFF, GIF)").to_string(),
        crate::backdrop::FILE_TYPES.to_vec(),
        then,
    );
}

/// Asks the browser for a file of the types `extensions`; it comes in a
/// later frame, with `then`.
fn ask_for_file(filter: String, extensions: Vec<&'static str>, then: Then) {
    wasm_bindgen_futures::spawn_local(async move {
        let Some(file) = rfd::AsyncFileDialog::new()
            .add_filter(&filter, &extensions)
            .pick_file()
            .await
        else {
            return;
        };
        let bytes = file.read().await;
        uploaded(file.file_name(), bytes, then);
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
        uploaded(name, bytes, Then::Open);
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

/// On a touch device, the first frame gets a touch that stops at once:
/// egui then knows the touch screen before the first real touch, so a
/// finger drags each scroll area from the start (egui scrolls with a drag
/// only on a touch screen), and the sizes for fingers apply.
pub fn touch_hint(raw: &mut eframe::egui::RawInput) {
    use eframe::egui::{Event, Pos2, TouchDeviceId, TouchId, TouchPhase};
    static DONE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if !DONE.swap(true, std::sync::atomic::Ordering::Relaxed) && touch_device() {
        raw.events.push(Event::Touch {
            device_id: TouchDeviceId(0),
            id: TouchId(0),
            phase: TouchPhase::Cancel,
            pos: Pos2::ZERO,
            force: None,
        });
    }
}

/// A pen (an Apple Pencil, a stylus) and the palm (the owner, 2026-11-23).
/// The page tells the program a pen by the type of its pointer events
/// ("pen"). While the pen is in use (down, or used in the last
/// [`PEN_HOLD_MS`], or a touch that started then is still down), the
/// touches on the screen are not input: a palm that rests on the screen
/// moves and taps nothing. The program gets the events of the pen itself,
/// as a mouse with one button. egui made the first touch on the screen the
/// pointer, so a palm that touched the screen first took the pointer from
/// the pen; filtering the touch events alone was not enough.
#[derive(Debug, Clone, Copy, PartialEq)]
enum PenEvent {
    /// Down, at (x, y) in the page (CSS pixels).
    Down(f32, f32),
    /// A move, down or over the screen.
    Move(f32, f32),
    Up(f32, f32),
    /// The system took the pen (a gesture of the system).
    Cancel,
}

#[derive(Debug, Default)]
struct Pen {
    /// The pen events since the last frame.
    events: Vec<PenEvent>,
    down: bool,
    /// The time of the last pen event (ms, `Date.now`).
    last_ms: f64,
    /// The touches that are down (their identifiers).
    touches: std::collections::BTreeSet<i32>,
    /// The touches that started while the pen was in use (a palm).
    palms: std::collections::BTreeSet<i32>,
    /// The last frame was in pen mode.
    active: bool,
    /// A long press of the pen: the right-click menu (`crate::pen`).
    press: crate::pen::LongPress,
}

/// After the pen goes up, touches are not input for this long (ms): the
/// palm rests on the screen between two strokes of the pen.
const PEN_HOLD_MS: f64 = 1000.0;
/// A pen that is down with no event for this long (ms) is up: the system
/// can take a stroke of the pen with no `pointerup` or `pointercancel`, and
/// a pen that stays down for the program makes the fingers do nothing.
const PEN_STALE_MS: f64 = 5000.0;

static PEN: Mutex<Pen> = Mutex::new(Pen {
    events: Vec::new(),
    down: false,
    last_ms: f64::NEG_INFINITY,
    touches: std::collections::BTreeSet::new(),
    palms: std::collections::BTreeSet::new(),
    active: false,
    press: crate::pen::LongPress::new(),
});

impl Pen {
    fn in_use(&self, now: f64) -> bool {
        (self.down && now - self.last_ms < PEN_STALE_MS)
            || now - self.last_ms < PEN_HOLD_MS
            || !self.palms.is_empty()
            || !self.events.is_empty()
    }
}

/// Listens to the pointer and touch events of the page before the canvas
/// does (capture on the window), to know the pen and the palm.
fn listen_for_pen() {
    use wasm_bindgen::closure::Closure;
    let Some(window) = web_sys::window() else { return };
    let options = web_sys::AddEventListenerOptions::new();
    options.set_capture(true);
    options.set_passive(true);
    for name in ["pointerdown", "pointermove", "pointerup", "pointercancel"] {
        let on = Closure::<dyn FnMut(web_sys::PointerEvent)>::new(move |e: web_sys::PointerEvent| {
            if e.pointer_type() != "pen" {
                return;
            }
            // The pen on a field of the page (handwriting) is for the field.
            let on_field = e
                .target()
                .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                .is_some_and(|t| t.class_name() == NATIVE_TEXT_CLASS);
            if on_field {
                return;
            }
            let (x, y) = (e.client_x() as f32, e.client_y() as f32);
            if let Ok(mut pen) = PEN.lock() {
                let event = match name {
                    "pointerdown" => {
                        pen.down = true;
                        PenEvent::Down(x, y)
                    }
                    "pointerup" => {
                        pen.down = false;
                        PenEvent::Up(x, y)
                    }
                    "pointercancel" => {
                        pen.down = false;
                        PenEvent::Cancel
                    }
                    _ => PenEvent::Move(x, y),
                };
                pen.events.push(event);
                pen.last_ms = js_sys::Date::now();
            }
            if let Some(ctx) = CONTEXT.get() {
                ctx.request_repaint();
            }
        });
        let _ = window.add_event_listener_with_callback_and_add_event_listener_options(
            name,
            on.as_ref().unchecked_ref(),
            &options,
        );
        on.forget();
    }
    for name in ["touchstart", "touchend", "touchcancel"] {
        let on = Closure::<dyn FnMut(web_sys::TouchEvent)>::new(move |e: web_sys::TouchEvent| {
            let Ok(mut pen) = PEN.lock() else { return };
            let in_use = pen.in_use(js_sys::Date::now());
            let changed = e.changed_touches();
            for k in 0..changed.length() {
                let Some(t) = changed.item(k) else { continue };
                let id = t.identifier();
                if name == "touchstart" {
                    pen.touches.insert(id);
                    if in_use {
                        pen.palms.insert(id);
                    }
                } else {
                    pen.touches.remove(&id);
                    pen.palms.remove(&id);
                }
            }
        });
        let _ = window.add_event_listener_with_callback_and_add_event_listener_options(
            name,
            on.as_ref().unchecked_ref(),
            &options,
        );
        on.forget();
    }
}

/// While a pen is in use: no touch input (the touches that are down end),
/// and the events of the pen as the pointer.
pub fn pen_filter(ctx: &eframe::egui::Context, raw: &mut eframe::egui::RawInput) {
    use eframe::egui::{Event, PointerButton, Pos2, TouchDeviceId, TouchId, TouchPhase};
    let Ok(mut pen) = PEN.lock() else { return };
    let now = js_sys::Date::now();
    if pen.down && now - pen.last_ms >= PEN_STALE_MS {
        pen.down = false;
        pen.events.push(PenEvent::Cancel);
    }
    if !pen.in_use(now) {
        pen.active = false;
        return;
    }
    let events = std::mem::take(&mut pen.events);
    raw.events.retain(|e| {
        !matches!(
            e,
            Event::Touch { .. } | Event::PointerMoved(_) | Event::PointerButton { .. } | Event::PointerGone
        )
    });
    if !pen.active {
        pen.active = true;
        for &id in &pen.touches {
            raw.events.push(Event::Touch {
                device_id: TouchDeviceId(0),
                id: TouchId::from(id),
                phase: TouchPhase::Cancel,
                pos: Pos2::ZERO,
                force: None,
            });
        }
    }
    // The place on the canvas, as eframe finds it for a mouse.
    let origin = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id("shokujodx"))
        .map(|c| c.get_bounding_client_rect())
        .map_or((0.0, 0.0), |r| (r.left() as f32, r.top() as f32));
    let zoom = ctx.zoom_factor();
    let at = |x: f32, y: f32| Pos2::new((x - origin.0) / zoom, (y - origin.1) / zoom);
    let modifiers = ctx.input(|i| i.modifiers);
    let button = |pos, pressed| Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers,
    };
    let mut last = None;
    for e in events {
        match e {
            PenEvent::Down(x, y) => {
                let p = at(x, y);
                raw.events.push(Event::PointerMoved(p));
                raw.events.push(button(p, true));
                pen.press.down(now, p);
                last = Some(p);
            }
            PenEvent::Move(x, y) => {
                let p = at(x, y);
                raw.events.push(Event::PointerMoved(p));
                pen.press.moved(p);
                last = Some(p);
            }
            PenEvent::Up(x, y) => {
                let p = at(x, y);
                raw.events.push(Event::PointerMoved(p));
                // After a long press the primary button is up already.
                if !pen.press.up() {
                    raw.events.push(button(p, false));
                }
                last = Some(p);
            }
            PenEvent::Cancel => {
                if let Some(p) = last
                    && !pen.press.up()
                {
                    raw.events.push(button(p, false));
                }
                raw.events.push(Event::PointerGone);
            }
        }
    }
    // A pen held still: the right-click menu, as a long press of a finger.
    if let Some(p) = pen.press.due(now) {
        raw.events.extend(crate::pen::menu_events(p, modifiers));
    }
}

/// A text field of the page over a text field of the program, on a touch
/// screen (the owner, 2026-11-25). egui types into a hidden field of one
/// pixel, and the handwriting of an iPad (Scribble) did not reach the text:
/// the letters written with the pen were lost, and the program stopped
/// answering. A real field of the page takes the handwriting, the keyboard,
/// dictation and corrections of the system; the program copies its text
/// each frame.
struct NativeText {
    /// The egui text field under it.
    id: eframe::egui::Id,
    element: web_sys::HtmlTextAreaElement,
    /// The text that the program has (to know which side changed it).
    known: String,
    /// The last pass that drew the field (`Context::cumulative_pass_nr`).
    pass: u64,
    /// The field of the page lost the focus: the user tapped somewhere else.
    blurred: std::rc::Rc<std::cell::Cell<bool>>,
    _on_blur: wasm_bindgen::closure::Closure<dyn FnMut()>,
}

impl Drop for NativeText {
    fn drop(&mut self) {
        self.element.remove();
    }
}

thread_local! {
    static NATIVE_TEXT: std::cell::RefCell<Option<NativeText>> = const { std::cell::RefCell::new(None) };
}

/// On a touch screen: while the egui text field of `response` has the
/// focus, a field of the page with the same `text` lies over it (see
/// [`NativeText`]). Call it after the text field, each frame. A field of
/// one line (`multiline` false) gets no new lines.
pub fn native_text(ui: &eframe::egui::Ui, response: &eframe::egui::Response, text: &mut String, multiline: bool) {
    if !touch_device() {
        return;
    }
    let ctx = ui.ctx();
    // The focus now: a `request_focus` after the field in this frame counts.
    let focused = ctx.memory(|m| m.has_focus(response.id));
    NATIVE_TEXT.with(|cell| {
        let mut slot = cell.borrow_mut();
        let ours = slot.as_ref().is_some_and(|n| n.id == response.id);
        if ours && (slot.as_ref().is_some_and(|n| n.blurred.get()) || !focused) {
            // The user tapped somewhere else, or closed the keyboard.
            *slot = None;
            response.surrender_focus();
            return;
        }
        if !focused {
            return;
        }
        if !ours {
            // Not while a pointer is down: the browser gives the canvas the
            // focus when that pointer goes up (eframe), and the field would
            // go at once (the tap that opened the window).
            if ui.input(|i| i.pointer.any_down() || i.any_touches()) {
                ctx.request_repaint();
                return;
            }
            *slot = make_native_text(response.id, text, ui.visuals());
        }
        let Some(n) = slot.as_mut() else { return };
        n.pass = ctx.cumulative_pass_nr();
        place_native_text(&n.element, response.rect, ctx);
        let mut value = n.element.value();
        if !multiline && value.contains(['\n', '\r']) {
            value = value.replace(['\n', '\r'], "");
            n.element.set_value(&value);
        }
        if value != n.known {
            text.clone_from(&value);
            n.known = value;
        } else if *text != n.known {
            n.element.set_value(text);
            n.known.clone_from(text);
        }
    });
}

/// A field of the page with `text`, with the colors of `visuals`, and the
/// focus.
fn make_native_text(id: eframe::egui::Id, text: &str, visuals: &eframe::egui::Visuals) -> Option<NativeText> {
    let document = web_sys::window()?.document()?;
    let element = document
        .create_element("textarea")
        .ok()?
        .dyn_into::<web_sys::HtmlTextAreaElement>()
        .ok()?;
    let css = |c: eframe::egui::Color32| format!("rgb({},{},{})", c.r(), c.g(), c.b());
    let style = element.style();
    for (k, v) in [
        ("position", "fixed".to_string()),
        ("z-index", "20".to_string()),
        ("box-sizing", "border-box".to_string()),
        ("margin", "0".to_string()),
        ("padding", "2px 4px".to_string()),
        ("resize", "none".to_string()),
        // 16 px or more: Safari does not zoom the page into the field.
        ("font", "16px system-ui, sans-serif".to_string()),
        ("background", css(visuals.extreme_bg_color)),
        ("color", css(visuals.text_color())),
        ("border", format!("1px solid {}", css(visuals.selection.stroke.color))),
        ("border-radius", "2px".to_string()),
        ("outline", "none".to_string()),
    ] {
        let _ = style.set_property(k, &v);
    }
    let _ = element.set_attribute("autocapitalize", "off");
    element.set_class_name(NATIVE_TEXT_CLASS);
    element.set_value(text);
    let blurred = std::rc::Rc::new(std::cell::Cell::new(false));
    let on_blur = {
        let blurred = blurred.clone();
        wasm_bindgen::closure::Closure::<dyn FnMut()>::new(move || {
            blurred.set(true);
            if let Some(ctx) = CONTEXT.get() {
                ctx.request_repaint();
            }
        })
    };
    let _ = element.add_event_listener_with_callback("blur", on_blur.as_ref().unchecked_ref());
    // Each change of the text: a frame, which copies it.
    let on_input = wasm_bindgen::closure::Closure::<dyn FnMut()>::new(move || {
        if let Some(ctx) = CONTEXT.get() {
            ctx.request_repaint();
        }
    });
    let _ = element.add_event_listener_with_callback("input", on_input.as_ref().unchecked_ref());
    on_input.forget();
    document.body()?.append_child(&element).ok()?;
    let options = web_sys::FocusOptions::new();
    options.set_prevent_scroll(true);
    let _ = element.focus_with_options(&options);
    let end = text.encode_utf16().count() as u32;
    let _ = element.set_selection_range(end, end);
    Some(NativeText {
        id,
        element,
        known: text.to_string(),
        pass: 0,
        blurred,
        _on_blur: on_blur,
    })
}

/// The class of the fields of [`NativeText`]: the pen events on them are
/// for the field, not for the program.
const NATIVE_TEXT_CLASS: &str = "shokujodx-native-text";

/// Puts the field of the page over `rect` (egui points) of the canvas.
fn place_native_text(element: &web_sys::HtmlTextAreaElement, rect: eframe::egui::Rect, ctx: &eframe::egui::Context) {
    let origin = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id("shokujodx"))
        .map(|c| c.get_bounding_client_rect())
        .map_or((0.0, 0.0), |r| (r.left() as f32, r.top() as f32));
    let zoom = ctx.zoom_factor();
    let style = element.style();
    for (k, v) in [
        ("left", origin.0 + rect.min.x * zoom),
        ("top", origin.1 + rect.min.y * zoom),
        ("width", rect.width() * zoom),
        ("height", rect.height() * zoom),
    ] {
        let _ = style.set_property(k, &format!("{v}px"));
    }
}

/// The field of the page goes when the program no longer draws its text
/// field (its window closed).
fn drop_stale_native_text(ctx: &eframe::egui::Context) {
    let now = ctx.cumulative_pass_nr();
    NATIVE_TEXT.with(|cell| {
        let mut slot = cell.borrow_mut();
        if slot.as_ref().is_some_and(|n| now > n.pass + 1) {
            *slot = None;
        }
    });
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
/// tools) and the quality Fast of the 3D look (less memory and work; View >
/// Thread Look > Quality changes it). Only with no saved settings, so a
/// choice of the user stays.
fn touch_defaults(st: &mut AppState) {
    if !FRESH.load(std::sync::atomic::Ordering::Relaxed) || !touch_device() {
        return;
    }
    let mut s = st.settings.clone();
    if page_width() < 700.0 {
        s.simple_mode = true;
        s.look.set_quality(crate::viewtools::Quality::Fast);
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
        let tiles = &st.stitch_tiles.stats;
        let stats = serde_json::json!({
            "mode": b.mode,
            "frames": b.frames,
            "mean_ms": b.total_ms / f64::from(b.frames),
            "slowest_ms": b.slowest_ms,
            "load_ms": b.load_ms,
            // The tiles: the longest job, the work of a million pixels of
            // the surface, and the tiles made in parts.
            "tile_ms": tiles.tile_ms,
            "tile_slowest_ms": tiles.slowest_ms,
            "tile_ms_per_mpx": tiles.ms_per_mpx,
            "tiles_in_parts": tiles.assembled,
            "tile_parts": tiles.assembly_parts,
        })
        .to_string();
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
    let rect = ctx.content_rect();
    // The middle of each object on the screen (points), for a test to tap.
    let d = st.doc.design();
    let middles: Vec<[f32; 2]> = st.canvas_rect.map_or_else(Vec::new, |r| {
        let map = crate::manipulate::ViewMap::new(r, st.view, d);
        d.objects()
            .iter()
            .filter_map(|o| d.bbox_of(&[o.id]))
            .map(|b| {
                let p = map.to_screen(b.center());
                [p.x, p.y]
            })
            .collect()
    });
    let state = serde_json::json!({
        "tool": st.tool.label(),
        "message": st.message,
        "zoom": st.view.zoom,
        "cx": st.view.center.x,
        "cy": st.view.center.y,
        "gesture": st.touch_gesture,
        "simple": st.settings.simple_mode,
        "text_scale": st.settings.text_scale,
        "undo": st.doc.undo_len(),
        "objects": st.doc.design().objects().len(),
        "kinds": st.doc.design().objects().iter().map(|o| o.type_name()).collect::<Vec<_>>(),
        "lettering": st.lettering_dialog.as_ref().map(|d| d.text.clone()),
        "selected": st.selection.len(),
        "middles": middles,
        "touching": touching,
        "middle": middle.map(|p| [p.x, p.y]),
        "canvas": st.canvas_rect.map(|r| [r.min.x, r.min.y, r.max.x, r.max.y]),
        "egui": [ctx.zoom_factor(), ctx.pixels_per_point(), rect.width(), rect.height()],
    });
    if let Some(w) = web_sys::window() {
        let _ = js_sys::Reflect::set(&w, &"shokujodxView".into(), &state.to_string().into());
    }
}

/// The build of a page: its version (a hash of the program), the time of
/// the build and the commit (`tools/build-web.py` writes them).
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize)]
#[serde(default)]
struct Build {
    version: String,
    built: String,
    commit: String,
}

/// The state of File > Check for Updates.
#[derive(Clone, Debug, PartialEq)]
enum PageCheck {
    Checking,
    /// The page is the newest build.
    Newest,
    /// A newer build is on the server.
    Newer(Build),
    Failed(String),
}

static PAGE_CHECK: Mutex<Option<PageCheck>> = Mutex::new(None);

/// The build of this page (`window.shokujodxBuild`; it does not change).
fn this_build() -> &'static Build {
    static BUILD: std::sync::OnceLock<Build> = std::sync::OnceLock::new();
    BUILD.get_or_init(|| {
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
    })
}

/// The result of the check: the newest build (the text of version.json)
/// against the build of this page.
fn compare(this: &Build, newest: &str) -> PageCheck {
    match serde_json::from_str::<Build>(newest) {
        Ok(n) if n.version.is_empty() => PageCheck::Failed("version.json has no version".into()),
        Ok(n) if n.version == this.version => PageCheck::Newest,
        Ok(n) => PageCheck::Newer(n),
        Err(e) => PageCheck::Failed(e.to_string()),
    }
}

/// The function `name` of the page (`window.name`), if the page has it.
fn page_function(name: &str) -> Option<(web_sys::Window, js_sys::Function)> {
    let w = web_sys::window()?;
    let f = js_sys::Reflect::get(&w, &name.into()).ok()?.dyn_into().ok()?;
    Some((w, f))
}

/// Calls the function `name` of the page (no arguments) and waits for its
/// promise: the text that it gives.
async fn call_page(name: &str) -> Result<String, String> {
    let (w, f) = page_function(name).ok_or("the page is older than the program")?;
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
/// build; the window shows the result (a new frame when it comes).
pub fn check_for_update(ctx: &eframe::egui::Context) {
    if let Ok(mut c) = PAGE_CHECK.lock() {
        *c = Some(PageCheck::Checking);
    }
    let ctx = ctx.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let result = match call_page("shokujodxNewest").await {
            Ok(text) => compare(this_build(), &text),
            Err(e) => PageCheck::Failed(e),
        };
        ctx.request_repaint();
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
        .fit_screen(ctx)
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
}

/// A design with more stitches than this is large for a phone or a tablet.
const LARGE_ON_TOUCH: usize = 150_000;

/// On a phone or a tablet, a large design gets a word one time (the design
/// with its document number): the memory of the page does not get smaller
/// while it is open, and the browser can close a page that has too much.
fn large_design_word(st: &mut AppState) {
    static TOLD: Mutex<Option<u64>> = Mutex::new(None);
    if !st.touch_seen || TOLD.lock().ok().and_then(|t| *t) == Some(st.doc_id) {
        return;
    }
    let stitches = st.shown_plan().plan.stitches.records.len();
    if stitches <= LARGE_ON_TOUCH {
        return;
    }
    if let Ok(mut t) = TOLD.lock() {
        *t = Some(st.doc_id);
    }
    st.message = crate::i18n::fill(
        "A large design for a phone or a tablet ({} stitches). The browser can close the page when its memory is full: save often (File > Save downloads the design), and reload the page to free the memory.",
        &[&stitches],
    );
}

/// Each frame: the measures, the uploaded files and the notices.
pub fn poll(st: &mut AppState, ctx: &eframe::egui::Context) {
    CONTEXT.get_or_init(|| ctx.clone());
    drop_stale_native_text(ctx);
    // A pen held still: a frame at the time of its long press. (A request
    // made before the pass, in `pen_filter`, is lost when the pass begins.)
    if PEN.lock().is_ok_and(|p| p.press.waiting()) {
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
    large_design_word(st);
    update_window(ctx);
    if !STARTED.swap(true, std::sync::atomic::Ordering::Relaxed) {
        first_frame(st);
    }
    bench_frame(st, ctx);
    probe(st, ctx);
    let uploads = UPLOADS.lock().map(|mut u| std::mem::take(&mut *u)).unwrap_or_default();
    for (name, bytes, then) in uploads {
        match then {
            Then::Open => st.open_upload(&name, bytes),
            Then::Backdrop => st.import_bitmap_bytes(bytes, &name),
            Then::Wizard => {
                st.import_bitmap_bytes(bytes, &name);
                if st.doc.design().backdrop.is_some() {
                    st.open_wizard();
                }
            }
            Then::Waiting => {
                let path = std::path::PathBuf::from(&name);
                FILE.with(|f| *f.borrow_mut() = Some((path.clone(), bytes)));
                if let Some(then) = WAITING.with(|w| w.borrow_mut().take()) {
                    then(st, path);
                }
            }
        }
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
    download_zip();
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
