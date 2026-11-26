//! Lessons (P19 Part 2 Milestone 6, ADR 0177): a tutorial that the user
//! does in the program, one step at a time.
//!
//! - A lesson does the step file of its tutorial (`steps.rs`). A step with
//!   `item` starts a **stage**: the stage shows that numbered item of the
//!   topic (and the items after it, up to the item of the next stage) in
//!   the language of the program, and has the steps up to the next stage.
//! - In a stage, the lesson marks what the user must click (a button of
//!   the ribbon or of a window, a thread, a point on the canvas), one step
//!   at a time. A step is done when the user did it (a click in the mark,
//!   a command that ran, a key, a value in a field) and the design is as
//!   its `expect` says (the checks that do not depend on the machine and
//!   the hand of the user, [`steps::Expect::misses`]). A stage with steps
//!   to do goes to the next stage by itself when it is done; a stage that
//!   the user only reads waits for **Next ▶**.
//! - **◀ Back**, **Skip ▶** and **Stop the Lesson**; the program keeps the place of a
//!   lesson that stops (`Settings::lessons`), and the Help window can go
//!   on from it.
//! - The lesson finds the widgets through the screen reader tree of egui,
//!   and sees the clicks and keys before the program takes them: the
//!   plugin [`Eyes`] (the screen reader tree is on while a lesson shows).
//!   The ribbon keeps the places of its buttons while a lesson shows
//!   (`ribbon::RECORD`).
//! - Steps with `test_only` are for the tests: the lesson does not mark
//!   them and does not wait for them.

use eframe::egui::{self, Align2, Color32, FontId, Key, PointerButton, Pos2, Rect, RichText, Stroke, Vec2};
use emb_geom::Point;

use crate::app::AppState;
use crate::help::{self, Block};
use crate::i18n::{fill, tr};
use crate::ribbon::{Cmd, MORE_KEY, button_rect};
use crate::steps::{self, Miss, Node, Screen, Step};

/// The id and the English title of the lesson window.
pub const WINDOW: &str = "Lesson";

/// The start of a help link that starts a lesson: `lesson:<id>` from the
/// start, `lesson:<id>@<stage>` from a stage.
pub const LINK: &str = "lesson:";

/// The windows whose widgets the lesson does not mark: their texts name
/// the buttons of the steps (the lesson window and the Help window).
pub fn quiet_windows() -> [egui::Id; 2] {
    [egui::Id::new(WINDOW), egui::Id::new("Help")]
}

/// A stage that the user did goes to the next stage after this time (s).
const NEXT_AFTER: f64 = 0.8;

/// A press and a release nearer than this are a click (points).
const CLICK_PX: f32 = 6.0;

/// The checks of the design of a step that is not done: at most this often
/// (s); the lesson asks for a frame after this time.
const CHECK_EVERY: f64 = 0.25;

/// The color of the marks.
const MARK: Color32 = Color32::from_rgb(255, 136, 0);

/// What a user did, as the program saw it before it took it.
#[derive(Debug, Clone)]
enum Input {
    Press(Pos2, PointerButton),
    Release(Pos2, PointerButton),
    Key(Key),
    /// Typed or pasted text.
    Text(String),
}

/// The eyes of the lessons: an egui plugin that keeps the widgets of the
/// screen reader tree of the last frame, and the clicks and keys of the
/// user before the program takes them.
#[derive(Default)]
pub struct Eyes {
    /// A lesson shows.
    on: bool,
    nodes: Vec<Node>,
    input: Vec<Input>,
    /// A screen reader tree came while no lesson showed (a screen reader
    /// uses it): the tree stays on after the lesson.
    tree_before: bool,
}

impl egui::plugin::Plugin for Eyes {
    fn debug_name(&self) -> &'static str {
        "lesson eyes"
    }

    fn input_hook(&mut self, _ctx: &egui::Context, input: &mut egui::RawInput) {
        if !self.on {
            return;
        }
        for e in &input.events {
            match e {
                egui::Event::PointerButton {
                    pos, button, pressed, ..
                } => self.input.push(if *pressed {
                    Input::Press(*pos, *button)
                } else {
                    Input::Release(*pos, *button)
                }),
                egui::Event::Key {
                    key,
                    pressed: true,
                    repeat: false,
                    ..
                } => self.input.push(Input::Key(*key)),
                egui::Event::Text(t) | egui::Event::Paste(t) => self.input.push(Input::Text(t.clone())),
                _ => {}
            }
        }
    }

    fn output_hook(&mut self, _ctx: &egui::Context, output: &mut egui::FullOutput) {
        if let Some(update) = &output.platform_output.accesskit_update {
            if self.on {
                self.nodes = steps::nodes_of(update);
            } else {
                self.tree_before = true;
            }
        }
    }
}

/// One stage of a lesson: an item of the topic and its steps.
#[derive(Debug, Clone)]
struct Stage {
    /// The English `##` heading of its section.
    heading: String,
    /// The number of the item (0: the whole section).
    item: u32,
    steps: Vec<Step>,
}

impl Stage {
    /// The user does something in this stage (else the user only reads it
    /// or the lesson only checks the design).
    fn acts(&self) -> bool {
        self.steps.iter().any(|s| !s.test_only && s.acts())
    }
}

/// A line of the text of a stage (Markdown).
#[derive(Debug, Clone, PartialEq)]
enum Line {
    Para(String),
    Bullet(String),
    Numbered(u32, String),
}

/// The texts of a lesson in one language.
#[derive(Debug, Clone, Default)]
struct Texts {
    lang: String,
    /// The title of the tutorial.
    title: String,
    /// The heading and the lines of each stage.
    stages: Vec<(String, Vec<Line>)>,
    /// The items of the section "Next" (links to other topics).
    next: Vec<String>,
}

/// What the user did since the step that is marked started.
#[derive(Debug, Clone, Default)]
struct Seen {
    /// A click in the mark of the step.
    hit: bool,
    /// Clicks on the canvas (not in a window).
    canvas_clicks: usize,
    /// Drags that started on the canvas.
    drags: usize,
    keys: Vec<Key>,
    /// The text that the user typed or pasted.
    typed: String,
    /// Commands that ran.
    ran: Vec<Cmd>,
    /// Start designs that opened.
    opened: Vec<String>,
}

/// What the lesson marks on the screen for a step.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Mark {
    /// A widget: its place, and if a click in it does the step (else it is
    /// the way to the widget: its tab, its folded panel, the arrow of its
    /// drop-down button).
    Widget(Rect, bool),
    /// Points on the canvas in their order: the place, the right button
    /// (a curve point); and the number of points done.
    Points(Vec<(Pos2, bool)>, usize),
    /// A drag on the canvas, from and to.
    Drag(Pos2, Pos2),
}

/// A lesson that shows.
#[derive(Debug)]
pub struct Lesson {
    /// The id of the tutorial.
    pub id: String,
    stages: Vec<Stage>,
    /// The stage that shows.
    pub at: usize,
    /// The step of the stage that is marked (the steps before it are done).
    step: usize,
    seen: Seen,
    /// The mark of the last frame (a click in it does the step).
    mark: Option<Mark>,
    /// The marked widget is off (a step before it is not done).
    mark_off: bool,
    /// The press of a click or a drag that goes on.
    press: Option<(Pos2, PointerButton)>,
    /// The windows of the last frame: a click there is not on the canvas
    /// (a menu that the click opens is not there yet).
    windows: Vec<Rect>,
    /// The time when the stage was done (it goes on after [`NEXT_AFTER`]).
    done_at: Option<f64>,
    /// The checks of the marked step that do not agree, and their time.
    misses: Vec<Miss>,
    checked_at: f64,
    texts: Texts,
    /// The user finished the last stage.
    pub finished: bool,
    /// The user stopped the lesson: it goes at the end of the frame.
    stopped: bool,
}

impl Lesson {
    /// The lesson of the tutorial `id`, at the stage `at`.
    pub fn new(id: &str, at: usize) -> Result<Lesson, String> {
        let text = steps::file(id).ok_or_else(|| fill("The tutorial {} has no lesson.", &[&id]))?;
        let all = steps::parse(text).map_err(|e| format!("{id}: {e}"))?;
        let mut stages: Vec<Stage> = Vec::new();
        for s in all.step {
            if let Some((heading, item)) = &s.item {
                stages.push(Stage {
                    heading: heading.clone(),
                    item: *item,
                    steps: Vec::new(),
                });
            }
            match stages.last_mut() {
                Some(stage) => stage.steps.push(s),
                None => return Err(format!("{id}: the first step has no item")),
            }
        }
        let mut lesson = Lesson {
            id: id.to_string(),
            at: at.min(stages.len().saturating_sub(1)),
            stages,
            step: 0,
            seen: Seen::default(),
            mark: None,
            mark_off: false,
            press: None,
            windows: Vec::new(),
            done_at: None,
            misses: Vec::new(),
            checked_at: f64::NEG_INFINITY,
            texts: Texts::default(),
            finished: false,
            stopped: false,
        };
        lesson.texts = lesson.make_texts("en")?;
        Ok(lesson)
    }

    /// A command ran (`AppState::run`).
    pub fn ran(&mut self, cmd: Cmd) {
        self.seen.ran.push(cmd);
    }

    /// A start design opened (`AppState::open_sample`).
    pub fn opened(&mut self, name: &str) {
        self.seen.opened.push(name.to_string());
    }

    /// Goes to the stage `at` (the end: the lesson is finished).
    fn go(&mut self, at: usize) {
        self.finished = at >= self.stages.len();
        self.at = at.min(self.stages.len().saturating_sub(1));
        self.step = 0;
        self.seen = Seen::default();
        self.mark = None;
        self.done_at = None;
        self.misses.clear();
        self.checked_at = f64::NEG_INFINITY;
    }

    /// The texts of the stages in the language `lang`: the items of the
    /// topic in that language (a translation has the blocks of the English
    /// topic, ADR 0084, so a section has the same place).
    fn make_texts(&self, lang: &str) -> Result<Texts, String> {
        let find = |code: &str| help::file_topics(code).into_iter().find(|t| t.id == self.id);
        let en = find("en").ok_or_else(|| format!("no topic {}", self.id))?;
        let here = find(lang).unwrap_or_else(|| en.clone());
        let heads = |body: &str| -> Vec<String> {
            help::blocks(body)
                .into_iter()
                .filter_map(|b| match b {
                    Block::Heading(2, t) => Some(t),
                    _ => None,
                })
                .collect()
        };
        let en_heads = heads(&en.body);
        // The sections of the topic of the language: the heading and the
        // blocks under it.
        let mut sections: Vec<(String, Vec<Block>)> = Vec::new();
        for b in help::blocks(&here.body) {
            match b {
                Block::Heading(2, t) => sections.push((t, Vec::new())),
                Block::Heading(1, _) => {}
                b => {
                    if let Some(s) = sections.last_mut() {
                        s.1.push(b);
                    }
                }
            }
        }
        if sections.len() != en_heads.len() {
            return Err(format!("{}: the topic in {lang} has other sections", self.id));
        }
        let index = |h: &str| {
            en_heads
                .iter()
                .position(|e| e == h)
                .ok_or_else(|| format!("{}: no section \"{h}\"", self.id))
        };
        let mut out = Texts {
            lang: lang.to_string(),
            title: here.title.clone(),
            ..Default::default()
        };
        for (k, stage) in self.stages.iter().enumerate() {
            let s = index(&stage.heading)?;
            let stop = self
                .stages
                .get(k + 1)
                .filter(|n| n.heading == stage.heading)
                .map_or(u32::MAX, |n| n.item);
            let (heading, blocks) = &sections[s];
            let mut lines = Vec::new();
            for b in blocks {
                match b {
                    Block::List(Some(first), items) => {
                        for (i, t) in items.iter().enumerate() {
                            let n = first + i as u32;
                            if stage.item == 0 || (stage.item..stop).contains(&n) {
                                lines.push(Line::Numbered(n, t.clone()));
                            }
                        }
                    }
                    Block::List(None, items) if stage.item == 0 => {
                        lines.extend(items.iter().map(|t| Line::Bullet(t.clone())));
                    }
                    Block::Para(t) if stage.item == 0 => lines.push(Line::Para(t.clone())),
                    _ => {}
                }
            }
            if stage.item != 0
                && !lines
                    .iter()
                    .any(|l| matches!(l, Line::Numbered(n, _) if *n == stage.item))
            {
                return Err(format!("{}: no item {} in \"{}\"", self.id, stage.item, stage.heading));
            }
            out.stages.push((heading.clone(), lines));
        }
        if let Ok(s) = index("Next") {
            for b in &sections[s].1 {
                if let Block::List(_, items) = b {
                    out.next.extend(items.iter().cloned());
                }
            }
        }
        Ok(out)
    }

    /// The steps of the stage that the lesson marks and waits for.
    fn stage(&self) -> &Stage {
        &self.stages[self.at]
    }

    /// The step that is marked, if the stage is not done.
    fn current(&self) -> Option<&Step> {
        self.stage().steps.get(self.step)
    }

    /// Takes the clicks and keys of the frame: a click in the mark of the
    /// last frame does the step of the mark.
    fn take_input(&mut self, input: Vec<Input>, canvas: Option<Rect>) {
        let windows = std::mem::take(&mut self.windows);
        let on_canvas = |p: Pos2| canvas.is_some_and(|c| c.contains(p)) && !windows.iter().any(|w| w.contains(p));
        for i in input {
            match i {
                Input::Press(p, b) => self.press = Some((p, b)),
                Input::Release(p, b) => {
                    let Some((from, pb)) = self.press.take() else { continue };
                    if pb != b {
                        continue;
                    }
                    if from.distance(p) < CLICK_PX {
                        if let Some(Mark::Widget(r, true)) = &self.mark
                            && r.expand(2.0).contains(p)
                        {
                            self.seen.hit = true;
                        }
                        if on_canvas(p) {
                            self.seen.canvas_clicks += 1;
                        }
                    } else if on_canvas(from) {
                        self.seen.drags += 1;
                    }
                }
                Input::Key(k) => self.seen.keys.push(k),
                Input::Text(t) => self.seen.typed.push_str(&t),
            }
        }
    }

    /// The user did the step `s` (the checks of its `expect` aside).
    fn did(&self, s: &Step, st: &AppState, screen: &Screen) -> bool {
        if s.test_only {
            return true;
        }
        let seen = &self.seen;
        let mut done = true;
        if let Some(name) = &s.open {
            done &= seen.opened.iter().any(|o| o == name);
        }
        if let Some(tab) = &s.tab {
            done &= st.settings.ribbon_tab.label() == tr(tab);
        }
        if let Some(place) = &s.command {
            let here: Vec<&str> = place.split(" > ").map(tr).collect();
            done &= match steps::command_at(&here) {
                Ok((cmd, _, _)) => seen.ran.contains(&cmd) || seen.hit,
                Err(_) => seen.hit,
            };
        }
        if let Some(name) = &s.thread {
            let d = st.doc.design();
            let now = st.current_thread.and_then(|id| d.thread(id)).map(|t| t.name.as_str());
            done &= now == Some(name.as_str()) || seen.hit;
        }
        if s.drag.is_some() {
            done &= seen.drags > 0;
        }
        if s.click.is_some() {
            done &= seen.canvas_clicks > 0;
        }
        if !s.clicks.is_empty() {
            done &= seen.canvas_clicks >= s.clicks.len();
        }
        if let Some(k) = &s.key {
            done &= Key::from_name(k).is_some_and(|k| seen.keys.contains(&k));
        }
        if s.button.is_some() || s.field.is_some() {
            done &= seen.hit;
        }
        if let Some(text) = &s.text {
            // In a field, or typed where no field shows it (the value of a
            // sketch dimension).
            let want = words(text);
            done &= words(&seen.typed).contains(&want)
                || screen
                    .nodes
                    .iter()
                    .any(|n| steps::TEXT_FIELDS.contains(&n.role) && words(&n.value).contains(&want));
        }
        if let Some([label, value]) = &s.set {
            let roles = [
                egui::accesskit::Role::SpinButton,
                egui::accesskit::Role::TextInput,
                egui::accesskit::Role::MultilineTextInput,
            ];
            done &= screen.after_label(tr(label), &roles).is_ok_and(|n| {
                let number = value.parse::<f64>().ok();
                match (n.number, number) {
                    (Some(a), Some(b)) => (a - b).abs() <= 1e-3 * b.abs().max(1.0),
                    _ => n.value.trim() == value.trim(),
                }
            });
        }
        if let Some([label, item]) = &s.choose {
            // The list shows the item, or the user clicked the item in the
            // open list (a list can show a short name of its items).
            let item = tr(item);
            done &= seen.hit
                || screen
                    .after_label(tr(label), &[egui::accesskit::Role::ComboBox])
                    .is_ok_and(|n| {
                        // The list can show the item with more (its size) or less.
                        let v = n.value.as_str();
                        !v.is_empty()
                            && (v == item || v.starts_with(&format!("{item} ")) || item.starts_with(&format!("{v} ")))
                    });
        }
        if let Some(title) = &s.close {
            done &= screen.in_window(tr(title)).is_none();
        }
        done
    }

    /// The mark of the step `s` on the screen of now.
    fn mark_of(s: &Step, st: &AppState, screen: &Screen) -> Option<Mark> {
        if s.test_only {
            return None;
        }
        let widget = |n: Result<Node, String>| n.ok().map(|n| Mark::Widget(n.rect, true));
        if let Some(tab) = &s.tab {
            return button_rect(tr(tab), false).map(|r| Mark::Widget(r, true));
        }
        if let Some(place) = &s.command {
            let english: Vec<&str> = place.split(" > ").collect();
            let here: Vec<&str> = english.iter().map(|p| tr(p)).collect();
            let (cmd, tab, panel) = steps::command_at(&here).ok()?;
            let label = cmd.meta().1;
            if english.len() > 1 && st.settings.ribbon_tab != tab && !st.settings.simple_mode {
                return button_rect(tab.label(), false).map(|r| Mark::Widget(r, false));
            }
            // In the menu of a drop-down button that is open.
            if button_rect(&format!("{MORE_KEY}{label}"), true).is_some()
                && let Some(n) = screen.in_window(label)
                && n.role != egui::accesskit::Role::Window
            {
                return Some(Mark::Widget(n.rect, true));
            }
            let folded = format!("{} {}", tr(panel), egui_phosphor::regular::CARET_DOWN);
            return button_rect(label, true)
                .map(|r| Mark::Widget(r, true))
                .or_else(|| button_rect(&format!("{MORE_KEY}{label}"), true).map(|r| Mark::Widget(r, false)))
                .or_else(|| button_rect(&folded, true).map(|r| Mark::Widget(r, false)));
        }
        if let Some(name) = &s.thread {
            return widget(screen.node(name));
        }
        let canvas = |p: [f64; 2]| -> Option<Pos2> {
            let rect = st.canvas_rect?;
            Some(crate::manipulate::ViewMap::new(rect, st.view, st.doc.design()).to_screen(Point::new(p[0], p[1])))
        };
        if let Some([a, b]) = s.drag {
            return Some(Mark::Drag(canvas(a)?, canvas(b)?));
        }
        if let Some(p) = s.click {
            return Some(Mark::Points(vec![(canvas(p)?, false)], 0));
        }
        if !s.clicks.is_empty() {
            let points: Option<Vec<(Pos2, bool)>> = s
                .clicks
                .iter()
                .map(|c| Some((canvas([*c.first()?, *c.get(1)?])?, c.get(2) == Some(&1.0))))
                .collect();
            return points.map(|p| Mark::Points(p, 0));
        }
        if let Some(text) = &s.button {
            return widget(screen.button_node(tr(text)));
        }
        if let Some(hint) = &s.field {
            return widget(screen.text_field(tr(hint)));
        }
        if let Some([label, _]) = &s.set {
            let roles = [
                egui::accesskit::Role::SpinButton,
                egui::accesskit::Role::TextInput,
                egui::accesskit::Role::MultilineTextInput,
            ];
            return widget(screen.after_label(tr(label), &roles));
        }
        if let Some([label, item]) = &s.choose {
            // The item, when its list is open; else the list.
            if let Some(n) = screen.list_item(tr(item)) {
                return Some(Mark::Widget(n.rect, true));
            }
            return screen
                .after_label(tr(label), &[egui::accesskit::Role::ComboBox])
                .ok()
                .map(|n| Mark::Widget(n.rect, false));
        }
        if let Some(title) = &s.close {
            return widget(screen.close_button(tr(title)));
        }
        None
    }

    /// What the user must do for the step `s` (Markdown), if anything.
    fn hint(s: &Step) -> Option<String> {
        if s.test_only {
            return None;
        }
        let place = |p: &str| p.split(" > ").map(tr).collect::<Vec<_>>().join(" > ");
        Some(if s.open.is_some() {
            tr("Click the link in the step to open the start design.").to_string()
        } else if let Some(tab) = &s.tab {
            fill("Click the tab **{}**.", &[&tr(tab)])
        } else if let Some(p) = &s.command {
            fill("Click **{}**.", &[&place(p)])
        } else if let Some(name) = &s.thread {
            fill("Click the thread **{}** in the Threads strip.", &[name])
        } else if s.drag.is_some() {
            tr("Drag from the first mark on the canvas to the second mark.").to_string()
        } else if s.click.is_some() {
            tr("Click the mark on the canvas.").to_string()
        } else if !s.clicks.is_empty() {
            tr("Click the marks on the canvas in their order: a dot with the left button, a ring with the right button.").to_string()
        } else if let Some(k) = &s.key {
            let name = if k == "Escape" { "Esc" } else { k.as_str() };
            fill("Push **{}**.", &[&name])
        } else if let Some(b) = &s.button {
            fill("Click **{}**.", &[&tr(b)])
        } else if let Some(f) = &s.field {
            fill("Click the field **{}**.", &[&tr(f)])
        } else if let Some(t) = &s.text {
            fill("Type `{}`.", &[&t.replace('\n', ", ")])
        } else if let Some([label, value]) = &s.set {
            fill("Set **{}** to {}.", &[&tr(label), value])
        } else if let Some([label, item]) = &s.choose {
            fill("In **{}**, choose **{}**.", &[&tr(label), &tr(item)])
        } else if let Some(title) = &s.close {
            fill("Close the window **{}**.", &[&tr(title)])
        } else if s.expect.is_some() {
            tr("The lesson checks the design.").to_string()
        } else {
            return None;
        })
    }

    /// Goes through the steps of the stage that are done, and to the next
    /// stage when a stage that the user did is done for a short time.
    fn advance(&mut self, st: &mut AppState, screen: &Screen, now: f64) {
        if self.finished {
            return;
        }
        while let Some(s) = self.current().cloned() {
            if !self.did(&s, st, screen) {
                self.misses.clear();
                return;
            }
            if let Some(e) = s.expect.as_ref().filter(|_| !s.test_only) {
                if now - self.checked_at < CHECK_EVERY && !self.misses.is_empty() {
                    return;
                }
                self.checked_at = now;
                self.misses = e.misses(st, screen, false);
                if !self.misses.is_empty() {
                    return;
                }
            }
            self.step += 1;
            self.seen = Seen::default();
            self.mark = None;
            self.misses.clear();
        }
        // The stage is done.
        if self.stage().acts() {
            let at = *self.done_at.get_or_insert(now);
            if now - at >= NEXT_AFTER {
                let next = self.at + 1;
                self.go(next);
            }
        }
    }
}

/// The words of a text (spaces and line breaks are one space).
fn words(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

impl AppState {
    /// Starts the lesson of the tutorial `id` at the stage `at` (0: the
    /// start). The Help window closes, so the canvas shows.
    pub fn start_lesson(&mut self, id: &str, at: usize) {
        match Lesson::new(id, at) {
            Ok(l) => {
                self.lesson = Some(l);
                self.help.open = false;
                self.focus_canvas = true;
                self.keep_lesson_place();
            }
            Err(e) => self.message = e,
        }
    }

    /// Keeps the place of the lesson in the settings (a finished lesson
    /// has none).
    fn keep_lesson_place(&mut self) {
        let Some(l) = &self.lesson else { return };
        let (id, at) = (l.id.clone(), (!l.finished).then_some(l.at));
        let mut s = self.settings.clone();
        let changed = match at {
            Some(at) => s.lessons.insert(id, at) != Some(at),
            None => s.lessons.remove(&id).is_some(),
        };
        if changed {
            self.set_settings(s);
        }
    }
}

/// The lesson of the frame: the eyes, the input, the steps that are done,
/// the marks and the window. Call it at the end of the frame, when the
/// canvas and the windows have their places.
pub fn frame(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut l) = st.lesson.take() else { return };
    if ctx.plugin_opt::<Eyes>().is_none() {
        ctx.add_plugin(Eyes::default());
    }
    if l.stopped {
        // The screen reader tree stays on for a screen reader.
        let keep = ctx
            .with_plugin(|e: &mut Eyes| {
                e.on = false;
                e.nodes.clear();
                e.input.clear();
                e.tree_before
            })
            .unwrap_or(true);
        if !keep {
            ctx.disable_accesskit();
        }
        crate::ribbon::RECORD.with(|r| r.set(cfg!(test)));
        return;
    }
    let (nodes, input) = ctx
        .with_plugin(|e: &mut Eyes| {
            if !e.on {
                e.on = true;
            }
            (std::mem::take(&mut e.nodes), std::mem::take(&mut e.input))
        })
        .unwrap_or_default();
    ctx.enable_accesskit();
    let screen = Screen {
        nodes,
        windows: steps::windows(ctx),
    }
    .without(ctx, &quiet_windows());
    let lang = crate::i18n::current();
    if l.texts.lang != lang {
        match l.make_texts(lang) {
            Ok(t) => l.texts = t,
            Err(e) => {
                st.message = e;
                l.texts.lang = lang.to_string();
            }
        }
    }
    let now = ctx.input(|i| i.time);
    let before = (l.at, l.finished);
    l.take_input(input, st.canvas_rect);
    l.windows = steps::windows(ctx);
    l.advance(st, &screen, now);
    l.mark = l.current().and_then(|s| Lesson::mark_of(s, st, &screen));
    if let Some(Mark::Points(_, done)) = &mut l.mark {
        *done = l.seen.canvas_clicks;
    }
    l.mark_off =
        matches!(l.mark, Some(Mark::Widget(r, true)) if screen.nodes.iter().any(|n| n.rect == r && !n.enabled));
    if st.kiosk.is_none() {
        draw_marks(ctx, l.mark.as_ref());
        window(ctx, st, &mut l);
    }
    // Look again soon: a check of the design that does not agree yet (a
    // job can change the design), or the next stage. A step that waits
    // for the user needs no frame: the input of the user makes one.
    if !l.finished && (l.done_at.is_some() || !l.misses.is_empty()) {
        ctx.request_repaint_after(std::time::Duration::from_secs_f64(CHECK_EVERY));
    }
    let changed = before != (l.at, l.finished) || l.stopped;
    st.lesson = Some(l);
    if changed {
        st.keep_lesson_place();
    }
}

/// Draws the mark over the program.
fn draw_marks(ctx: &egui::Context, mark: Option<&Mark>) {
    let Some(mark) = mark else { return };
    let p = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("lesson marks")));
    let stroke = Stroke::new(3.0, MARK);
    let ring = |c: Pos2, filled: bool, strong: bool| {
        let color = if strong { MARK } else { MARK.gamma_multiply(0.35) };
        p.circle_stroke(c, 10.0, Stroke::new(3.0, color));
        if filled {
            p.circle_filled(c, 3.5, color);
        }
    };
    match mark {
        Mark::Widget(r, _) => {
            p.rect_stroke(r.expand(4.0), 5.0, stroke, egui::StrokeKind::Outside);
            p.rect_filled(r.expand(4.0), 5.0, MARK.gamma_multiply(0.12));
        }
        Mark::Points(points, done) => {
            for (i, (c, right)) in points.iter().enumerate() {
                ring(*c, !right, i >= *done);
                if points.len() > 1 {
                    p.text(
                        *c + Vec2::new(12.0, -12.0),
                        Align2::LEFT_BOTTOM,
                        (i + 1).to_string(),
                        FontId::proportional(14.0),
                        if i >= *done { MARK } else { MARK.gamma_multiply(0.35) },
                    );
                }
            }
        }
        Mark::Drag(a, b) => {
            ring(*a, true, true);
            ring(*b, true, true);
            p.arrow(*a, (*b - *a) * 0.9, stroke);
        }
    }
}

/// The lesson window: the stage, what to do, and Back, Skip or Next, the
/// tutorial and Stop.
fn window(ctx: &egui::Context, st: &mut AppState, l: &mut Lesson) {
    // The id is the English title ([`WINDOW`]; F1 opens its topic).
    let id = egui::Id::new("Lesson");
    let mut open = true;
    let mut link: Option<String> = None;
    let mut go: Option<usize> = None;
    let win = crate::dock::docked_at(
        egui::Window::new(tr("Lesson")).id(id),
        ctx,
        id,
        st.canvas_rect,
        Align2::RIGHT_BOTTOM,
    );
    win.open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(360.0)
        .show(ctx, |ui| {
            ui.set_max_width(360.0);
            ui.label(RichText::new(&l.texts.title).strong());
            if l.finished {
                ui.add_space(6.0);
                ui.label(RichText::new(tr("You did all the steps of the lesson.")).color(ok_color(ui)));
                if !l.texts.next.is_empty() {
                    ui.add_space(4.0);
                    ui.label(tr("Next:"));
                    for item in &l.texts.next {
                        if let Some(to) = help::show_spans(ui, item, false) {
                            link = Some(to);
                        }
                    }
                }
                ui.add_space(6.0);
                if ui.button(tr("Close")).clicked() {
                    l.stopped = true;
                }
                return;
            }
            let n = l.stages.len();
            ui.horizontal(|ui| {
                ui.weak(fill("Step {} of {}", &[&(l.at + 1), &n]));
                if let Some((heading, _)) = l.texts.stages.get(l.at) {
                    ui.weak("·");
                    ui.weak(heading);
                }
            });
            ui.add(egui::ProgressBar::new((l.at as f32 + 1.0) / n.max(1) as f32).desired_height(4.0));
            ui.add_space(4.0);
            if let Some((_, lines)) = l.texts.stages.get(l.at) {
                for line in lines {
                    let shown = ui.horizontal_top(|ui| {
                        match line {
                            Line::Numbered(k, _) => {
                                ui.add_sized([18.0, 14.0], egui::Label::new(format!("{k}.")));
                            }
                            Line::Bullet(_) => {
                                ui.add_sized([18.0, 14.0], egui::Label::new("•"));
                            }
                            Line::Para(_) => {}
                        }
                        let (Line::Numbered(_, t) | Line::Bullet(t) | Line::Para(t)) = line;
                        help::show_spans(ui, t, false)
                    });
                    if let Some(to) = shown.inner {
                        link = Some(to);
                    }
                }
            }
            ui.add_space(4.0);
            let done = l.current().is_none();
            if done {
                if l.stage().steps.iter().any(|s| s.expect.is_some() || (!s.test_only && s.acts())) {
                    ui.label(RichText::new(format!("✔ {}", tr("Done."))).color(ok_color(ui)));
                }
            } else if let Some(hint) = l.current().and_then(Lesson::hint) {
                egui::Frame::new()
                    .fill(MARK.gamma_multiply(0.15))
                    .corner_radius(4.0)
                    .inner_margin(6.0)
                    .show(ui, |ui| {
                        ui.horizontal_top(|ui| {
                            ui.label(RichText::new("▶").color(MARK));
                            help::show_spans(ui, &hint, false);
                        });
                    });
            }
            if l.mark_off {
                ui.label(RichText::new(tr("The marked button is off now: a step before it is not done. Click Back, or do that step.")).weak());
            }
            if !l.misses.is_empty() {
                let warn = ui.visuals().warn_fg_color;
                for m in &l.misses {
                    ui.label(RichText::new(m.text()).color(warn));
                }
                ui.label(RichText::new(tr("Click Undo and do the step again, or click Skip.")).weak());
            }
            // The pictures of the stage: what it looks like.
            for name in l.stage().steps.iter().filter_map(|s| s.picture.as_deref()) {
                if let Some(tex) = help::picture_texture(ui.ctx(), name) {
                    let size = tex.size_vec2();
                    let scale = (ui.available_width() / size.x).min(0.5);
                    ui.add(egui::Image::new((tex.id(), size * scale)));
                }
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.add_enabled(l.at > 0, egui::Button::new(tr("◀ Back"))).clicked() {
                    go = Some(l.at - 1);
                }
                let next = if done && !l.stage().acts() {
                    tr("Next ▶")
                } else {
                    tr("Skip ▶")
                };
                if ui
                    .button(next)
                    .on_hover_text(tr("Go to the next step. Undo takes back what the steps did."))
                    .clicked()
                {
                    go = Some(l.at + 1);
                }
                if ui
                    .button(tr("Tutorial"))
                    .on_hover_text(tr("Open the tutorial of this lesson in the Help window."))
                    .clicked()
                {
                    let heading = l.texts.stages.get(l.at).map(|(h, _)| h.clone());
                    st.open_help(&l.id, heading.as_deref());
                }
                if ui
                    .button(tr("Stop the Lesson"))
                    .on_hover_text(tr("Stop the lesson. The program keeps its place: the tutorial in the Help window can go on from it."))
                    .clicked()
                {
                    l.stopped = true;
                }
            });
        });
    if !open {
        l.stopped = true;
    }
    if let Some(at) = go {
        l.go(at);
    }
    if let Some(to) = link {
        match to.strip_prefix(crate::samples::LINK) {
            Some(name) => {
                l.opened(name);
                st.open_sample(name);
            }
            None => st.open_help(&to, None),
        }
    }
}

#[cfg(test)]
impl Mark {
    /// The point `p` is in the mark: in the widget, or at a point of the
    /// canvas (the start of a drag).
    pub fn has(&self, p: Pos2) -> bool {
        match self {
            Mark::Widget(r, _) => r.expand(4.0).contains(p),
            Mark::Points(points, _) => points.iter().any(|(c, _)| c.distance(p) < 3.0),
            Mark::Drag(a, _) => a.distance(p) < 3.0,
        }
    }
}

/// What the tests see of a lesson.
#[cfg(test)]
impl Lesson {
    /// The stage and the step that is marked (the number of stages when
    /// the lesson is finished).
    pub fn position(&self) -> (usize, usize) {
        if self.finished {
            (self.stages.len(), 0)
        } else {
            (self.at, self.step)
        }
    }

    /// The number of stages.
    pub fn len(&self) -> usize {
        self.stages.len()
    }

    /// The stage is done and the user only reads it: it waits for Next.
    pub fn waits_for_next(&self) -> bool {
        !self.finished && self.current().is_none() && !self.stage().acts()
    }

    /// The stage is done and goes on by itself.
    pub fn goes_on(&self) -> bool {
        !self.finished && self.current().is_none() && self.stage().acts()
    }

    /// The mark of the last frame.
    pub fn mark(&self) -> Option<Mark> {
        self.mark.clone()
    }

    /// The checks of the marked step that do not agree (English).
    pub fn misses(&self) -> Vec<String> {
        self.misses.iter().map(Miss::english).collect()
    }
}

fn ok_color(ui: &egui::Ui) -> Color32 {
    if ui.visuals().dark_mode {
        Color32::from_rgb(110, 200, 120)
    } else {
        Color32::from_rgb(30, 120, 50)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_window_id_is_its_english_title() {
        assert_eq!(WINDOW, "Lesson");
        assert!(help::WINDOW_HELP.contains(&(WINDOW, help::TUTORIALS_ID)));
    }

    /// Each step file is a lesson: its stages are items of its topic, in
    /// each language.
    #[test]
    fn each_step_file_is_a_lesson() {
        for (id, _) in steps::HELP_STEPS {
            let mut l = Lesson::new(id, 0).unwrap_or_else(|e| panic!("{e}"));
            assert!(l.len() > 1, "{id}");
            for lang in ["de", "es", "ja"] {
                l.texts = l.make_texts(lang).unwrap_or_else(|e| panic!("{lang}: {e}"));
                assert_eq!(l.texts.stages.len(), l.len());
            }
        }
    }

    /// A stage shows its item and the items after it, up to the item of
    /// the next stage in its section; a stage of item 0 shows the section.
    #[test]
    fn a_stage_shows_its_items() {
        let l = Lesson::new("tutorial-first-design", 0).unwrap();
        let t = &l.texts;
        let numbers = |k: usize| -> Vec<u32> {
            t.stages[k]
                .1
                .iter()
                .filter_map(|l| match l {
                    Line::Numbered(n, _) => Some(*n),
                    _ => None,
                })
                .collect()
        };
        assert_eq!(t.stages[0].0, "Open the start design");
        assert_eq!(numbers(0), [1, 2], "the second item only says what shows");
        assert_eq!(numbers(1), [1]);
        let check = t.stages.iter().position(|(h, _)| h == "Check it").unwrap();
        assert!(t.stages[check].1.iter().any(|l| matches!(l, Line::Bullet(_))));
        assert!(!t.next.is_empty(), "the Next links");
    }
}
