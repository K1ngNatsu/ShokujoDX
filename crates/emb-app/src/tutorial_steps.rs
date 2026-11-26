//! The tutorials as steps that the program does (P19 Part 2, ADR 0162).
//! Test code only: nothing of this file is in the program that users get.
//!
//! - `docs/help/steps/<id>.toml` has the steps of the tutorial `<id>`
//!   (`steps.rs`): the clicks and keys of a user, the checks of the design,
//!   and the pictures of the topic. [`Runner`] does them in the whole
//!   program (the ribbon, the windows, the canvas) with no screen, with the
//!   studio of `pictures.rs`. A step names a command by its place in
//!   English (`Digitize > Shapes > Ellipse`) and a button by its English
//!   text; the runner finds them in the language of the run.
//! - `each_lesson_follows_its_steps` starts the lesson of each tutorial
//!   (`lesson.rs`, ADR 0177) and does the steps: the lesson marks the
//!   widget or the point that each step clicks, sees each step done, and
//!   comes to its end.
//! - `each_tutorial_runs_in_each_language` does each step file in each
//!   language. A step that does not find its command, its button or its
//!   window, or a check that does not agree, makes the test fail.
//! - `the_places_in_the_tutorials_are_in_the_program` is the quick check
//!   of all tutorials: each bold place with " > " is a tab, a panel, a
//!   command or an item of the File menu, in the language of the topic.
//! - `write_tutorial_pictures` (ignored) writes the pictures of the
//!   tutorials to `docs/help/pictures`: a picture of the canvas has no text
//!   and is one file; a picture of a window is one file for each language.
//!
//! ```text
//! cargo test --release -p emb-app -- --ignored write_tutorial_pictures
//! ```

use std::path::{Path, PathBuf};

use eframe::egui::accesskit::Role;
use eframe::egui::{self, Event, Key, Modifiers, PointerButton, Pos2, Rect, Vec2};
use emb_geom::Point;

use crate::app::EmbApp;
use crate::i18n::tr;
use crate::manipulate::ViewMap;
use crate::pictures::{Picture, Studio, base};
use crate::ribbon::{RibbonTab, SelInfo, button_rect};
use crate::steps::{Expect, Node, Screen, Step, TEXT_FIELDS, command_at, nodes_of};

/// The languages of the help.
const LANGUAGES: [&str; 4] = ["en", "de", "es", "ja"];

/// A screen so wide that no panel of the ribbon folds (a folded panel
/// stays open as a menu, and shows in the pictures of windows).
const SCREEN: Vec2 = Vec2::new(3840.0, 1600.0);

/// The step files (built into the program): the id of the tutorial and
/// the text of its steps.
fn step_files() -> Vec<(String, String)> {
    crate::steps::HELP_STEPS
        .iter()
        .map(|(id, text)| (id.to_string(), text.to_string()))
        .collect()
}

/// Checks a place of a topic (in the language of now): a command (see
/// [`command_at`]), `Tab > Panel`, or `File > Item` (`file` is the File
/// menu). The "…" at the end of an item is not needed.
fn check_place(place: &[&str], file: &[String]) -> Result<(), String> {
    if place.first() == Some(&tr("File")) {
        let item = |s: &str| s.trim_end_matches('…').to_string();
        return match place {
            [_, name] if file.iter().any(|f| item(f) == item(name)) => Ok(()),
            _ => Err(format!("no File menu item \"{}\"", place.join(" > "))),
        };
    }
    if let [tab, panel] = place
        && let Some(t) = RibbonTab::ALL.iter().find(|t| t.label() == *tab)
        && crate::ribbon::help_panels(*t, &egui::Context::default())
            .iter()
            .any(|p| p.title == *panel)
    {
        return Ok(());
    }
    command_at(place).map(|_| ())
}

/// The widgets of the screen reader tree of a frame.
fn tree(out: &egui::FullOutput) -> Vec<Node> {
    out.platform_output
        .accesskit_update
        .as_ref()
        .map(nodes_of)
        .unwrap_or_default()
}

/// A picture that a step made: its name, if it has text (a picture for
/// each language), and the picture (`None` when the run does not paint).
struct Made {
    name: String,
    text: bool,
    picture: Option<Picture>,
}

/// The whole program in one language, and the steps that it did.
struct Runner {
    studio: Studio,
    app: EmbApp,
    nodes: Vec<Node>,
    /// Paint the pictures (else only find their areas).
    paint: bool,
    made: Vec<Made>,
    /// The places of the clicks and the starts of the drags of the step.
    clicked: Vec<Pos2>,
}

impl Runner {
    fn new(lang: &str, paint: bool) -> Runner {
        crate::i18n::set_language(lang);
        let mut st = base(lang);
        st.save_settings = false;
        let studio = Studio::new();
        studio.ctx.enable_accesskit();
        let mut r = Runner {
            studio,
            app: EmbApp::for_test(st),
            nodes: Vec::new(),
            paint,
            made: Vec::new(),
            clicked: Vec::new(),
        };
        r.settle();
        r
    }

    fn frame(&mut self, events: Vec<Event>) {
        let out = self.studio.input(&mut self.app, SCREEN, events);
        self.nodes = tree(&out);
    }

    /// Frames until the windows have their sizes and the background work
    /// of the program is done (or a short time went by).
    fn settle(&mut self) {
        for i in 0..40 {
            let out = self.studio.input(&mut self.app, SCREEN, Vec::new());
            self.nodes = tree(&out);
            let wait = out
                .viewport_output
                .get(&egui::ViewportId::ROOT)
                .map_or(std::time::Duration::MAX, |v| v.repaint_delay);
            if i >= 3 {
                if wait > std::time::Duration::from_millis(300) {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
        }
        // The background work of the Picture to Design wizard (at most two
        // minutes, as its tests), and the search for the fonts of the
        // computer (the font lists change when it ends).
        let start = std::time::Instant::now();
        while (self
            .app
            .state()
            .wizard
            .as_ref()
            .is_some_and(|w| w.working() || w.editing())
            || self.app.state().fonts.scanning())
            && start.elapsed() < std::time::Duration::from_secs(120)
        {
            std::thread::sleep(std::time::Duration::from_millis(20));
            let out = self.studio.input(&mut self.app, SCREEN, Vec::new());
            self.nodes = tree(&out);
        }
    }

    fn press(&mut self, at: Pos2, mods: Modifiers, pressed: bool) -> Event {
        let _ = self;
        Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed,
            modifiers: mods,
        }
    }

    fn click_at(&mut self, at: Pos2, mods: Modifiers) {
        self.click_with(at, mods, PointerButton::Primary);
    }

    fn click_with(&mut self, at: Pos2, mods: Modifiers, button: PointerButton) {
        self.clicked.push(at);
        let ev = |pressed| Event::PointerButton {
            pos: at,
            button,
            pressed,
            modifiers: mods,
        };
        let (down, up) = (ev(true), ev(false));
        self.frame(vec![Event::ModifiersChanged(mods), Event::PointerMoved(at)]);
        self.frame(vec![down]);
        self.frame(vec![up]);
        self.frame(vec![Event::ModifiersChanged(Modifiers::NONE)]);
        self.settle();
    }

    fn drag(&mut self, from: Pos2, to: Pos2, mods: Modifiers) {
        self.clicked.push(from);
        let (down, up) = (self.press(from, mods, true), self.press(to, mods, false));
        self.frame(vec![Event::ModifiersChanged(mods), Event::PointerMoved(from)]);
        self.frame(vec![down]);
        for i in 1..=10 {
            let t = i as f32 / 10.0;
            self.frame(vec![Event::PointerMoved(from + (to - from) * t)]);
        }
        self.frame(vec![up]);
        self.frame(vec![Event::ModifiersChanged(Modifiers::NONE)]);
        self.settle();
    }

    fn key(&mut self, key: Key) {
        let ev = |pressed| Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: Modifiers::NONE,
        };
        self.frame(vec![ev(true)]);
        self.frame(vec![ev(false)]);
        self.settle();
    }

    /// The place on the screen of a point of the design (mm).
    fn point_at(&mut self, p: [f64; 2]) -> Result<Pos2, String> {
        let st = self.app.state();
        let rect = st.canvas_rect.ok_or("the canvas does not show")?;
        Ok(ViewMap::new(rect, st.view, st.doc.design()).to_screen(Point::new(p[0], p[1])))
    }

    /// Sets the field after `label` to `value`: a click on it, all its
    /// text selected, the value typed, Enter.
    fn set_field(&mut self, label: &str, value: &str) -> Result<(), String> {
        let f = self
            .screen()
            .after_label(label, &[Role::SpinButton, Role::TextInput, Role::MultilineTextInput])?;
        if !f.enabled {
            return Err(format!("the field \"{label}\" is off"));
        }
        self.click_select_all(&f);
        self.frame(vec![Event::Text(value.to_string())]);
        self.key(Key::Enter);
        Ok(())
    }

    /// A click on the text field `f`, then Ctrl+A: what is typed next
    /// replaces its text.
    fn click_select_all(&mut self, f: &Node) {
        self.click_at(f.rect.center(), Modifiers::NONE);
        let all = |pressed| Event::Key {
            key: Key::A,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: Modifiers::COMMAND,
        };
        self.frame(vec![Event::ModifiersChanged(Modifiers::COMMAND), all(true)]);
        self.frame(vec![all(false), Event::ModifiersChanged(Modifiers::NONE)]);
    }

    /// Chooses `item` in the list after `label`. A long list that does
    /// not show the item gets the item typed in its search field, or, with
    /// none, scrolls down with the wheel of the mouse (as a user does).
    fn choose(&mut self, label: &str, item: &str) -> Result<(), String> {
        let list = self.screen().after_label(label, &[Role::ComboBox])?;
        // Not the lesson window: it changes its size when a step is done.
        let before = self.screen().windows;
        let fields: Vec<Rect> = self
            .nodes
            .iter()
            .filter(|n| TEXT_FIELDS.contains(&n.role))
            .map(|n| n.rect)
            .collect();
        self.click_at(list.rect.center(), Modifiers::NONE);
        // The open list: the area that was not there before the click (it
        // opens under the list button, or over it near the bottom). An item
        // shows when it is all in it (not under the window behind it).
        let open = self.screen().windows.into_iter().find(|w| !before.contains(w));
        let shown = |r: &Runner| {
            r.screen()
                .list_item(item)
                .filter(|n| open.is_none_or(|o| o.expand(1.0).contains_rect(n.rect)))
        };
        if shown(self).is_none() {
            // The search field of the list: a field that came with it (not
            // a field of the window under it).
            let search = self
                .screen()
                .text_field(tr("Search"))
                .ok()
                .filter(|f| !fields.contains(&f.rect));
            if let Some(search) = search {
                self.click_select_all(&search);
                self.frame(vec![Event::Text(item.to_string())]);
                self.settle();
            } else {
                let at = open.map_or(list.rect.center_bottom() + Vec2::new(0.0, 40.0), |r| r.center());
                for _ in 0..80 {
                    if shown(self).is_some() {
                        break;
                    }
                    self.frame(vec![
                        Event::PointerMoved(at),
                        Event::MouseWheel {
                            unit: egui::MouseWheelUnit::Point,
                            delta: Vec2::new(0.0, -30.0),
                            phase: egui::TouchPhase::Move,
                            modifiers: Modifiers::NONE,
                        },
                    ]);
                    // Frames until the list stops (it scrolls smoothly).
                    for _ in 0..4 {
                        self.frame(Vec::new());
                    }
                }
            }
        }
        let n = shown(self).ok_or_else(|| {
            format!(
                "no item \"{item}\" in the list \"{label}\" (the windows have {:?})",
                self.screen().window_names()
            )
        })?;
        self.click_at(n.rect.center(), Modifiers::NONE);
        Ok(())
    }

    fn click_tab(&mut self, label: &str) -> Result<(), String> {
        let rect = button_rect(label, false).ok_or_else(|| format!("no tab \"{label}\""))?;
        self.click_at(rect.center(), Modifiers::NONE);
        Ok(())
    }

    /// Does one step.
    fn step(&mut self, s: &Step) -> Result<(), String> {
        let mods = holding(&s.holding)?;
        self.clicked.clear();
        if let Some(file) = &s.font_file {
            let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(file);
            self.app.state().fonts.add_computer_font(&path)?;
        }
        if let Some(name) = &s.open {
            if crate::samples::design(name).is_none() {
                return Err(format!("no start design \"{name}\""));
            }
            self.app.state().open_sample(name);
            self.settle();
        }
        if let Some(tab) = &s.tab {
            self.click_tab(tr(tab))?;
        }
        if let Some(place) = &s.command {
            let english: Vec<&str> = place.split(" > ").collect();
            let here: Vec<&str> = english.iter().map(|p| tr(p)).collect();
            let (cmd, tab, panel) = command_at(&here)?;
            if english.len() > 1 {
                self.click_tab(tab.label())?;
            }
            let label = cmd.meta().1;
            let st = self.app.state();
            if !st.can(cmd, &SelInfo::of(st)) {
                return Err(format!("the command \"{place}\" can not run now"));
            }
            if !s.check_only {
                // A panel that does not fit is one button; a click on it
                // shows the panel.
                let more = format!("{}{label}", crate::ribbon::MORE_KEY);
                if button_rect(label, true).is_none() && button_rect(&more, true).is_none() {
                    let folded = format!("{} {}", tr(panel), egui_phosphor::regular::CARET_DOWN);
                    let rect = button_rect(&folded, true).ok_or_else(|| format!("no button \"{label}\""))?;
                    self.click_at(rect.center(), Modifiers::NONE);
                }
                match button_rect(label, true) {
                    Some(rect) => self.click_at(rect.center(), Modifiers::NONE),
                    // A command under the arrow of a drop-down button: the
                    // arrow, then the command in its menu.
                    None => {
                        let arrow = button_rect(&more, true).ok_or_else(|| format!("no button \"{label}\""))?;
                        self.click_at(arrow.center(), Modifiers::NONE);
                        let item = self
                            .screen()
                            .in_window(label)
                            .ok_or_else(|| format!("no \"{label}\" under the arrow"))?;
                        self.click_at(item.rect.center(), Modifiers::NONE);
                    }
                }
            }
        }
        if let Some(name) = &s.thread {
            let n = self.screen().node(name)?;
            self.click_at(n.rect.center(), Modifiers::NONE);
        }
        if let Some([a, b]) = s.drag {
            let (from, to) = (self.point_at(a)?, self.point_at(b)?);
            self.drag(from, to, mods);
        }
        if let Some(p) = s.click {
            let at = self.point_at(p)?;
            self.click_at(at, mods);
        }
        for c in &s.clicks {
            let (button, p) = match c.as_slice() {
                [x, y] => (PointerButton::Primary, [*x, *y]),
                [x, y, r] if *r == 1.0 => (PointerButton::Secondary, [*x, *y]),
                _ => return Err(format!("a click is [x, y] or [x, y, 1], not {c:?}")),
            };
            let at = self.point_at(p)?;
            self.click_with(at, mods, button);
        }
        if let Some(k) = &s.key {
            let key = Key::from_name(k).ok_or_else(|| format!("no key \"{k}\""))?;
            self.key(key);
        }
        if let Some(text) = &s.button {
            let n = self.screen().button_node(tr(text))?;
            if !n.enabled {
                return Err(format!("the button \"{text}\" is off"));
            }
            self.click_at(n.rect.center(), Modifiers::NONE);
        }
        if let Some(hint) = &s.field {
            let f = self.screen().text_field(tr(hint))?;
            self.click_select_all(&f);
        }
        if let Some(text) = &s.text {
            self.frame(vec![Event::Text(text.clone())]);
            self.settle();
        }
        if let Some([label, value]) = &s.set {
            self.set_field(tr(label), value)?;
        }
        if let Some([label, item]) = &s.choose {
            self.choose(tr(label), tr(item))?;
        }
        if let Some(title) = &s.close {
            let n = self.screen().close_button(tr(title))?;
            self.click_at(n.rect.center(), Modifiers::NONE);
            if self.screen().in_window(tr(title)).is_some() {
                return Err(format!("the window \"{title}\" does not close"));
            }
        }
        if let Some(e) = &s.expect {
            self.check(e)?;
        }
        if let Some(name) = &s.picture {
            self.picture(name, s)?;
        }
        Ok(())
    }

    fn check(&mut self, e: &Expect) -> Result<(), String> {
        let screen = self.screen();
        let st = self.app.state();
        let mut bad: Vec<String> = e.misses(st, &screen, true).iter().map(|m| m.english()).collect();
        if bad.is_empty() {
            return Ok(());
        }
        if !st.message.is_empty() {
            bad.push(format!("the status line says \"{}\"", st.message));
        }
        Err(bad.join("; "))
    }

    fn picture(&mut self, name: &str, s: &Step) -> Result<(), String> {
        // No pointer: no tooltip and no hover in the picture.
        self.frame(vec![Event::PointerGone]);
        self.settle();
        let area = s.area.as_deref().unwrap_or("canvas");
        let (rect, text) = match area {
            "canvas" => {
                let [a, b] = match s.around {
                    Some(around) => around,
                    None => {
                        let d = self.app.state().doc.design();
                        let product = d.product.as_ref().filter(|v| v.visible).map(|v| v.bbox());
                        let e = match (d.extents(), product) {
                            (Some(e), Some(p)) => e.union(p),
                            (e, p) => e.or(p).ok_or("the design is empty")?,
                        };
                        let e = e.expand(5.0);
                        [[e.min.x, e.min.y], [e.max.x, e.max.y]]
                    }
                };
                let (a, b) = (self.point_at(a)?, self.point_at(b)?);
                let canvas = self.app.state().canvas_rect.ok_or("the canvas does not show")?;
                (Rect::from_two_pos(a, b).intersect(canvas), false)
            }
            "windows" => {
                let windows = self.windows();
                let all = windows.into_iter().reduce(Rect::union).ok_or("no window shows")?;
                (all.expand(8.0), true)
            }
            "panel" => {
                let title = s.panel.as_deref().ok_or("a picture of a panel needs \"panel\"")?;
                let key = format!("{}{}", crate::ribbon::PANEL_KEY, tr(title));
                let r = button_rect(&key, true).ok_or_else(|| format!("no panel \"{title}\""))?;
                (r.expand(6.0), true)
            }
            "widgets" => {
                let mut all: Option<Rect> = None;
                for w in &s.widgets {
                    let r = self.screen().node(tr(w))?.rect;
                    all = Some(all.map_or(r, |a| a.union(r)));
                }
                (all.ok_or("a picture of widgets needs \"widgets\"")?.expand(8.0), true)
            }
            _ => return Err(format!("no area \"{area}\"")),
        };
        let rect = rect.intersect(Rect::from_min_size(Pos2::ZERO, SCREEN));
        if rect.width() < 40.0 || rect.height() < 40.0 {
            return Err(format!("the picture \"{name}\" is too small: {rect:?}"));
        }
        let picture = self.paint.then(|| {
            let shot = self.studio.shot(&mut self.app, SCREEN);
            fit_width(shot.picture.crop(rect))
        });
        self.made.push(Made {
            name: name.to_string(),
            text,
            picture,
        });
        Ok(())
    }

    /// The windows and menus that show (in points).
    fn windows(&self) -> Vec<Rect> {
        crate::steps::windows(&self.studio.ctx)
    }

    /// The widgets of the last frame and the windows that show (not the
    /// lesson window and the Help window, as the lesson sees them).
    fn screen(&self) -> Screen {
        Screen {
            nodes: self.nodes.clone(),
            windows: self.windows(),
        }
        .without(&self.studio.ctx, &crate::lesson::quiet_windows())
    }

    /// The items of the File menu (their texts, in the language of now).
    fn file_menu(&mut self) -> Result<Vec<String>, String> {
        let file = button_rect("File", false).ok_or("no File button")?;
        self.click_at(file.center(), Modifiers::NONE);
        let menu = self
            .windows()
            .into_iter()
            .reduce(Rect::union)
            .ok_or("the File menu does not open")?;
        // The name of an item is its icon, two spaces, its text, a space
        // and its keys (as "Ctrl+S"), or the arrow of a menu.
        let items = self
            .nodes
            .iter()
            .filter(|n| menu.contains_rect(n.rect))
            .map(|n| {
                let text = n.name.split_once("  ").map_or(n.name.as_str(), |(_, t)| t);
                let text = text.trim_end().trim_end_matches('⏵').trim_end();
                match text.rsplit_once(' ') {
                    Some((head, keys)) if keys.contains('+') => head.to_string(),
                    _ => text.to_string(),
                }
            })
            .collect();
        self.key(Key::Escape);
        Ok(items)
    }
}

/// The widest picture in the help, in pixels: the Help window shows it
/// at this size or smaller.
const MAX_PICTURE_W: u32 = 720;

/// `p`, made smaller to [`MAX_PICTURE_W`] if it is wider.
fn fit_width(p: Picture) -> Picture {
    let w = p.width as u32;
    if w <= MAX_PICTURE_W {
        return p;
    }
    let bytes: Vec<u8> = p.pixels.iter().flatten().copied().collect();
    let Some(img) = image::RgbaImage::from_raw(w, p.height as u32, bytes) else {
        return p;
    };
    let h = (f64::from(p.height as u32) * f64::from(MAX_PICTURE_W) / f64::from(w)).round() as u32;
    let small = image::imageops::resize(&img, MAX_PICTURE_W, h.max(1), image::imageops::FilterType::Triangle);
    Picture {
        width: small.width() as usize,
        height: small.height() as usize,
        pixels: small.pixels().map(|px| px.0).collect(),
    }
}

/// The modifiers of the names in `holding`.
fn holding(names: &[String]) -> Result<Modifiers, String> {
    let mut m = Modifiers::NONE;
    for n in names {
        m |= match n.as_str() {
            "ctrl" => Modifiers::COMMAND,
            "shift" => Modifiers::SHIFT,
            "alt" => Modifiers::ALT,
            _ => return Err(format!("no key \"{n}\" to hold")),
        };
    }
    Ok(m)
}

/// Does the steps of the tutorial `id` in `lang`: the pictures that they
/// made, or the first step that went wrong.
fn run(id: &str, text: &str, lang: &str, paint: bool) -> Result<Vec<Made>, String> {
    let steps = crate::steps::parse(text).map_err(|e| format!("{id}: {e}"))?;
    // The steps as the file writes them, for the messages.
    let written: Vec<String> = toml::from_str::<toml::Table>(text)
        .ok()
        .and_then(|t| t.get("step")?.as_array().cloned())
        .unwrap_or_default()
        .iter()
        .map(|v| v.to_string().replace('\n', " "))
        .collect();
    let mut r = Runner::new(lang, paint);
    for (i, s) in steps.step.iter().enumerate() {
        let shown = written.get(i).map_or("", String::as_str);
        r.step(s)
            .map_err(|e| format!("{id} in {lang}, step {} ({shown}): {e}", i + 1))?;
    }
    crate::i18n::set_language("en");
    Ok(r.made)
}

/// The step needs an event of the user (a click, a key, a command that
/// runs), so a lesson cannot see it done before the user does it.
fn needs_event(s: &Step) -> bool {
    s.open.is_some()
        || s.command.is_some()
        || s.drag.is_some()
        || s.click.is_some()
        || !s.clicks.is_empty()
        || s.key.is_some()
        || s.button.is_some()
        || s.field.is_some()
}

impl Runner {
    /// The lesson: its stage and step, if it waits for Next, if it goes
    /// on by itself.
    fn lesson(&mut self) -> Result<((usize, usize), bool, bool), String> {
        let l = self.app.state().lesson.as_ref().ok_or("the lesson stopped")?;
        Ok((l.position(), l.waits_for_next(), l.goes_on()))
    }

    /// Clicks Next in the lesson window (and waits for a stage that goes on
    /// by itself) until the lesson is at the stage `k`, or after it.
    fn lesson_to(&mut self, k: usize) -> Result<(), String> {
        let mut idle = 0;
        for _ in 0..400 {
            let ((at, step), waits, goes_on) = self.lesson()?;
            let finished = self.app.state().lesson.as_ref().is_some_and(|l| l.finished);
            if at >= k || finished {
                return Ok(());
            }
            if waits {
                idle = 0;
                let next = self.screen_all().button_node(tr("Next ▶"))?;
                self.click_at(next.rect.center(), Modifiers::NONE);
            } else if goes_on || idle < 10 {
                // A stage that goes on by itself, or a new stage that the
                // lesson looks at in the next frame.
                idle += usize::from(!goes_on);
                self.frame(Vec::new());
            } else {
                let misses = self.app.state().lesson.as_ref().map(|l| l.misses()).unwrap_or_default();
                return Err(format!("the lesson waits at stage {} step {step} {misses:?}", at + 1));
            }
        }
        Err("the lesson does not go on".into())
    }

    /// The widgets of the last frame with the lesson window.
    fn screen_all(&self) -> Screen {
        Screen {
            nodes: self.nodes.clone(),
            windows: self.windows(),
        }
    }
}

/// Does the steps of the tutorial `id` in `lang` with its lesson open: the
/// lesson must mark the widget or the point that each step of the user
/// clicks, see the step done after it, and come to its end.
fn run_lesson(id: &str, text: &str, lang: &str) -> Result<(), String> {
    let steps = crate::steps::parse(text).map_err(|e| format!("{id}: {e}"))?;
    // The place of each step in the lesson: its stage and its index there.
    let mut places = Vec::new();
    let mut stage: Option<usize> = None;
    let mut j = 0;
    for s in &steps.step {
        if s.item.is_some() {
            stage = Some(stage.map_or(0, |k| k + 1));
            j = 0;
        }
        places.push((stage.ok_or(format!("{id}: the first step has no item"))?, j));
        j += 1;
    }
    let mut r = Runner::new(lang, false);
    r.app.state().start_lesson(id, 0);
    if r.app.state().lesson.is_none() {
        return Err(format!("{id}: no lesson: {}", r.app.state().message));
    }
    r.settle();
    for (i, s) in steps.step.iter().enumerate() {
        let err = |e: String| format!("{id} in {lang}, step {} (stage {}): {e}", i + 1, places[i].0 + 1);
        let (k, j) = places[i];
        let user = !s.test_only && s.acts();
        // The mark of the lesson before the step (the step is marked).
        let mut mark = None;
        let mut marked = false;
        if user {
            r.lesson_to(k).map_err(err)?;
            let (at, _, _) = r.lesson().map_err(err)?;
            if needs_event(s) && at > (k, j) {
                return Err(err(format!(
                    "the lesson went past the step before it was done ({at:?})"
                )));
            }
            if at == (k, j) {
                marked = true;
                mark = r.app.state().lesson.as_ref().and_then(|l| l.mark());
            }
        }
        r.step(s).map_err(|e| format!("{id} in {lang}, step {}: {e}", i + 1))?;
        // A command that opens a window of the system: the test does not
        // click it, a user does (the lesson sees it run).
        if user && s.check_only {
            let place = s.command.as_deref().unwrap_or_default();
            let here: Vec<&str> = place.split(" > ").map(tr).collect();
            let (cmd, _, _) = command_at(&here).map_err(err)?;
            if let Some(l) = &mut r.app.state().lesson {
                l.ran(cmd);
            }
            r.frame(Vec::new());
        }
        if user {
            // A click of the step is in the mark.
            if marked && !r.clicked.is_empty() && !s.check_only {
                match &mark {
                    Some(m) if r.clicked.iter().any(|p| m.has(*p)) => {}
                    Some(m) => return Err(err(format!("the lesson marks {m:?}, the step clicks {:?}", r.clicked))),
                    None => {
                        return Err(err(format!(
                            "the lesson marks nothing, the step clicks {:?}",
                            r.clicked
                        )));
                    }
                }
            }
            // The lesson sees the step done (a check can take a few frames).
            let mut passed = false;
            for _ in 0..120 {
                let (at, _, _) = r.lesson().map_err(err)?;
                if at > (k, j) {
                    passed = true;
                    break;
                }
                r.frame(Vec::new());
            }
            if !passed {
                let (at, _, _) = r.lesson().map_err(err)?;
                let misses = r.app.state().lesson.as_ref().map(|l| l.misses()).unwrap_or_default();
                return Err(err(format!(
                    "the lesson does not see the step done; it is at {at:?} {misses:?}"
                )));
            }
        }
    }
    r.lesson_to(usize::MAX)
        .map_err(|e| format!("{id} in {lang}, at the end: {e}"))?;
    crate::i18n::set_language("en");
    Ok(())
}

/// The bold places with " > " in a Markdown text (a place can go over two
/// lines).
fn bold_places(body: &str) -> Vec<String> {
    let flat = body.split_whitespace().collect::<Vec<_>>().join(" ");
    flat.split("**")
        .skip(1)
        .step_by(2)
        .filter(|b| b.contains(" > "))
        .map(str::to_string)
        .collect()
}

fn pictures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/help/pictures")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::help::{self, Block, Group};

    /// The picture names in the English topic `id`.
    fn topic_pictures(id: &str) -> Vec<String> {
        let topic = help::file_topics("en")
            .into_iter()
            .find(|t| t.id == id)
            .unwrap_or_else(|| panic!("no topic {id}"));
        help::blocks(&topic.body)
            .into_iter()
            .filter_map(|b| match b {
                Block::Picture(_, name) => Some(name),
                _ => None,
            })
            .collect()
    }

    /// Each step file does its steps in each language, and makes the
    /// pictures that its topic shows. The tutorials of the first path
    /// have steps (P19 Part 2, Milestone 1).
    #[test]
    fn each_tutorial_runs_in_each_language() {
        let files = step_files();
        let topics = help::file_topics("en");
        for t in topics.iter().filter(|t| t.card.path.as_deref() == Some("first-steps")) {
            assert!(files.iter().any(|(id, _)| *id == t.id), "{} has no step file", t.id);
        }
        let mut errors = Vec::new();
        // `EMB_TUTORIAL=<id>`: only this tutorial (while you write it).
        let only = std::env::var("EMB_TUTORIAL").unwrap_or_default();
        for (id, text) in files.iter().filter(|(id, _)| only.is_empty() || *id == only) {
            assert!(topics.iter().any(|t| t.id == *id), "no topic for the steps {id}");
            let shown = topic_pictures(id);
            for lang in LANGUAGES {
                match run(id, text, lang, false) {
                    Ok(made) => {
                        let names: Vec<&String> = made.iter().map(|m| &m.name).collect();
                        let want: Vec<&String> = shown.iter().collect();
                        if names != want {
                            errors.push(format!(
                                "{id} in {lang}: the steps make {names:?}, the topic shows {want:?}"
                            ));
                        }
                    }
                    Err(e) => errors.push(e),
                }
            }
        }
        crate::i18n::set_language("en");
        assert!(errors.is_empty(), "{}", errors.join("\n"));
    }

    /// A command that is not at its place makes a step fail; so a changed
    /// name of a command makes the test of its tutorial fail.
    #[test]
    fn a_step_with_a_wrong_place_fails() {
        let good = "[[step]]\nopen = \"first-steps\"\n[[step]]\ncommand = \"Digitize > Shapes > Ellipse\"\nexpect = { on = \"Digitize > Shapes > Ellipse\" }\n";
        run("good", good, "en", false).unwrap();
        for wrong in [
            "Digitize > Shapes > Oval",
            "Digitize > Fills > Ellipse",
            "Draw > Shapes > Ellipse",
        ] {
            let text = format!("[[step]]\ncommand = \"{wrong}\"\n");
            let e = run("wrong", &text, "en", false).err().expect("the steps fail");
            assert!(e.contains("no command at"), "{wrong}: {e}");
        }
        let e = run("wrong", "[[step]]\nbutton = \"No such button\"\n", "en", false)
            .err()
            .expect("the steps fail");
        assert!(e.contains("no widget"), "{e}");
        let e = run(
            "wrong",
            "[[step]]\nopen = \"first-steps\"\nexpect = { objects = 3 }\n",
            "en",
            false,
        )
        .err()
        .expect("the steps fail");
        assert!(e.contains("0 objects, not 3"), "{e}");
        assert!(run("wrong", "[[step]]\nclik = [0.0, 0.0]\n", "en", false).is_err());
        crate::i18n::set_language("en");
    }

    /// The lesson of each tutorial follows its steps (P19 Part 2 Milestone
    /// 6, ADR 0177): it marks what each step of the user clicks, sees each
    /// step done, and comes to its end. In English for all tutorials, and
    /// in Japanese for the first one (the names of the marks are found in
    /// the language of the program).
    #[test]
    fn each_lesson_follows_its_steps() {
        let only = std::env::var("EMB_TUTORIAL").unwrap_or_default();
        let mut errors = Vec::new();
        for (id, text) in step_files().iter().filter(|(id, _)| only.is_empty() || *id == only) {
            let langs: &[&str] = if id == "tutorial-first-design" {
                &["en", "ja"]
            } else {
                &["en"]
            };
            for lang in langs {
                if let Err(e) = run_lesson(id, text, lang) {
                    errors.push(e);
                }
            }
        }
        crate::i18n::set_language("en");
        assert!(
            errors.is_empty(),
            "{}",
            errors.join(
                "
"
            )
        );
    }

    /// The quick check: each bold place with " > " in a tutorial (in each
    /// language) is a tab and a panel, a command or an item of the File
    /// menu of the program.
    #[test]
    fn the_places_in_the_tutorials_are_in_the_program() {
        let mut errors = Vec::new();
        for lang in LANGUAGES {
            let mut r = Runner::new(lang, false);
            let file = r.file_menu().unwrap_or_else(|e| panic!("{lang}: {e}"));
            assert!(file.iter().any(|f| f == tr("Save")), "{lang}: {file:?}");
            for t in help::file_topics(lang).iter().filter(|t| t.group == Group::Tutorials) {
                for place in bold_places(&t.body) {
                    let parts: Vec<&str> = place.split(" > ").collect();
                    if let Err(e) = check_place(&parts, &file) {
                        errors.push(format!("{lang} {}: {e}", t.id));
                    }
                }
            }
        }
        crate::i18n::set_language("en");
        assert!(errors.is_empty(), "{}", errors.join("\n"));
    }

    #[test]
    fn bold_places_are_found_over_two_lines() {
        let body = "Click **Digitize > Shapes\n   > Ellipse**, then **Esc** and **Design >\nFabric**.";
        assert_eq!(bold_places(body), ["Digitize > Shapes > Ellipse", "Design > Fabric"]);
    }

    /// Writes the pictures of the tutorials to `docs/help/pictures` (the
    /// canvas from the English run, the windows in each language).
    #[test]
    #[ignore = "writes the pictures of the tutorials; run it by hand in a release build"]
    fn write_tutorial_pictures() {
        let dir = pictures_dir();
        let mut written = 0;
        let only = std::env::var("EMB_TUTORIAL").unwrap_or_default();
        for (id, text) in step_files()
            .into_iter()
            .filter(|(id, _)| only.is_empty() || *id == only)
        {
            for lang in LANGUAGES {
                let made = run(&id, &text, lang, true).unwrap_or_else(|e| panic!("{e}"));
                for m in made {
                    let path = match (m.text, lang) {
                        (false, "en") => dir.join(format!("{}.png", m.name)),
                        (false, _) => continue,
                        (true, _) => dir.join(lang).join(format!("{}.png", m.name)),
                    };
                    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                    m.picture.expect("painted").save(&path).unwrap();
                    println!("{}", path.display());
                    written += 1;
                }
            }
        }
        crate::i18n::set_language("en");
        assert!(written > 0);
    }
}
