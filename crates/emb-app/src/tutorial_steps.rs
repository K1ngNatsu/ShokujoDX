//! The tutorials as steps that the program does (P19 Part 2, ADR 0162).
//! Test code only: nothing of this file is in the program that users get.
//!
//! - `docs/help/steps/<id>.toml` has the steps of the tutorial `<id>`: the
//!   clicks and keys of a user, the checks of the design, and the pictures
//!   of the topic. [`Runner`] does them in the whole program (the ribbon,
//!   the windows, the canvas) with no screen, with the studio of
//!   `pictures.rs`. A step names a command by its place in English
//!   (`Digitize > Shapes > Ellipse`) and a button by its English text; the
//!   runner finds them in the language of the run.
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
use serde::Deserialize;

use crate::app::EmbApp;
use crate::i18n::tr;
use crate::manipulate::ViewMap;
use crate::pictures::{Picture, Studio, base};
use crate::ribbon::{Cmd, RibbonTab, SelInfo, all_commands};

/// The languages of the help.
const LANGUAGES: [&str; 4] = ["en", "de", "es", "ja"];

/// The roles of text fields: the name of such a widget is its text.
const TEXT_FIELDS: [Role; 3] = [Role::TextInput, Role::MultilineTextInput, Role::SearchInput];

/// A screen so wide that no panel of the ribbon folds (a folded panel
/// stays open as a menu, and shows in the pictures of windows).
const SCREEN: Vec2 = Vec2::new(3840.0, 1600.0);

/// The steps of one tutorial.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Steps {
    step: Vec<Step>,
}

/// One step: one of the actions, a check, or a picture.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, default)]
struct Step {
    /// Open the start design of this name (as its link in the topic).
    open: Option<String>,
    /// Click a tab of the ribbon (its English name).
    tab: Option<String>,
    /// Click a command: `Tab > Panel > Command`, `Tab > Command`, or the
    /// name of a command next to the File button.
    command: Option<String>,
    /// The command opens a window of the system (a file or a folder): the
    /// step only checks that it is there and can run.
    check_only: bool,
    /// Click the color of the thread of this name in the Threads strip.
    thread: Option<String>,
    /// Drag on the canvas from the first point to the second (mm).
    drag: Option<[[f64; 2]; 2]>,
    /// Click on the canvas at this point (mm).
    click: Option<[f64; 2]>,
    /// Clicks on the canvas, one after the other (mm): `[x, y]` with the
    /// left button (a corner), `[x, y, 1]` with the right button (a curve
    /// point).
    clicks: Vec<Vec<f64>>,
    /// The keys held for `drag` and `click`: `ctrl`, `shift`, `alt`.
    holding: Vec<String>,
    /// Push a key: `Enter`, `Escape`, `Delete`.
    key: Option<String>,
    /// Click the button with this English text (in a window or a menu
    /// if more than one widget has the name). With no widget of that name,
    /// a widget in a window whose name starts with the text and a space
    /// (a row of a list: "Graduate Regular  ·  TrueType").
    button: Option<String>,
    /// Click the text field with this English hint text (a search field,
    /// a field with no label), or the text field after this English label.
    /// Then `type` types into it.
    field: Option<String>,
    /// Add this font file (a path from the folder of the repository) to the
    /// fonts, as a font of the computer: the step files do not depend on
    /// the fonts of the computer that runs them.
    font_file: Option<String>,
    /// Close the window with this English title.
    close: Option<String>,
    /// Type this text (into the field that has the keyboard).
    #[serde(rename = "type")]
    text: Option<String>,
    /// Set the field after the English label to the value: `["Height",
    /// "25"]` (a number field or a text field on the row of the label).
    set: Option<[String; 2]>,
    /// Choose an item of the list after the English label: `["Baseline",
    /// "Arc over"]` (the item in English too).
    choose: Option<[String; 2]>,
    /// Check the design and the program.
    expect: Option<Expect>,
    /// Make the picture of this name.
    picture: Option<String>,
    /// The part of the screen in the picture: `canvas` (with `around`),
    /// `windows` (the windows that are open), `panel` (a panel of the
    /// ribbon, with `panel`) or `widgets` (with `widgets`).
    area: Option<String>,
    /// The English title of the ribbon panel in a picture of a panel.
    panel: Option<String>,
    /// The part of the design in a picture of the canvas (mm, two corners);
    /// with none, the whole design and 5 mm round it.
    around: Option<[[f64; 2]; 2]>,
    /// The widgets in a picture of the area `widgets` (English names).
    widgets: Vec<String>,
}

/// What the design and the program must be after the steps before.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, default)]
struct Expect {
    /// The number of objects.
    objects: Option<usize>,
    /// The kinds of the objects in sew order (`ObjectKind::type_name`).
    kinds: Option<Vec<String>>,
    /// The names of the threads of the objects in sew order.
    threads: Option<Vec<String>>,
    /// The number of colors that sew.
    colors: Option<usize>,
    /// The number of stitches is in this range.
    stitches: Option<[usize; 2]>,
    /// The number of trims (the status line).
    trims: Option<usize>,
    /// The width and the height of the design (mm, within 0.5 mm).
    size: Option<[f64; 2]>,
    /// The number of selected objects.
    selected: Option<usize>,
    /// A widget with this English name shows (a button, a label).
    shows: Option<String>,
    /// A window or a menu with this English title shows.
    window: Option<String>,
    /// The command at this place is on (a view, a tool).
    on: Option<String>,
    /// The id of the fabric of the design.
    fabric: Option<String>,
    /// The number of warnings and problems of Design Info.
    warnings: Option<usize>,
    /// The number of holes of all fills.
    holes: Option<usize>,
    /// The number of hoopings (Hooping mode).
    hoopings: Option<usize>,
    /// The number of rows of the team names.
    names: Option<usize>,
}

/// The step files: the id of the tutorial and the text of its steps.
fn step_files() -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/help/steps");
    let mut out: Vec<(String, String)> = std::fs::read_dir(&dir)
        .map(|d| {
            d.filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().is_some_and(|x| x == "toml"))
                .filter_map(|p| {
                    let id = p.file_stem()?.to_string_lossy().into_owned();
                    Some((id, std::fs::read_to_string(&p).ok()?))
                })
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out
}

/// The command at `place` (in the language of now): `Tab > Panel >
/// Command`, `Tab > Command`, or the name of a command next to the File
/// button. Also its tab and the English title of its panel.
fn command_at(place: &[&str]) -> Result<(Cmd, RibbonTab, &'static str), String> {
    let all = all_commands();
    let label = |c: Cmd| c.meta().1;
    let found = match place {
        [name] => all
            .iter()
            .find(|(c, _, panel)| *panel == tr("Quick access") && label(*c) == *name),
        [tab, name] => all.iter().find(|(c, t, _)| t.label() == *tab && label(*c) == *name),
        [tab, panel, name] => all
            .iter()
            .find(|(c, t, p)| t.label() == *tab && tr(p) == *panel && label(*c) == *name),
        _ => None,
    };
    found
        .map(|(c, t, p)| (*c, *t, *p))
        .ok_or_else(|| format!("no command at \"{}\"", place.join(" > ")))
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

/// A widget for a screen reader: its name (empty if it has none), its
/// place and its kind.
#[derive(Debug, Clone)]
struct Node {
    name: String,
    /// The hint text of a text field.
    placeholder: String,
    rect: Rect,
    enabled: bool,
    role: Role,
}

fn nodes_of(out: &egui::FullOutput) -> Vec<Node> {
    let Some(update) = &out.platform_output.accesskit_update else {
        return Vec::new();
    };
    update
        .nodes
        .iter()
        .filter_map(|(_, n)| {
            let b = n.bounds()?;
            Some(Node {
                name: n.label().or(n.value()).unwrap_or_default().to_string(),
                placeholder: n.placeholder().unwrap_or_default().to_string(),
                rect: Rect::from_min_max(Pos2::new(b.x0 as f32, b.y0 as f32), Pos2::new(b.x1 as f32, b.y1 as f32)),
                enabled: !n.is_disabled(),
                role: n.role(),
            })
        })
        .collect()
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
        };
        r.settle();
        r
    }

    fn frame(&mut self, events: Vec<Event>) {
        let out = self.studio.input(&mut self.app, SCREEN, events);
        self.nodes = nodes_of(&out);
    }

    /// Frames until the windows have their sizes and the background work
    /// of the program is done (or a short time went by).
    fn settle(&mut self) {
        for i in 0..40 {
            let out = self.studio.input(&mut self.app, SCREEN, Vec::new());
            self.nodes = nodes_of(&out);
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
            self.nodes = nodes_of(&out);
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
    fn screen(&mut self, p: [f64; 2]) -> Result<Pos2, String> {
        let st = self.app.state();
        let rect = st.canvas_rect.ok_or("the canvas does not show")?;
        Ok(ViewMap::new(rect, st.view, st.doc.design()).to_screen(Point::new(p[0], p[1])))
    }

    /// The place of the ribbon button `label` in the last frame: the first
    /// one (a tab) or the last one (a command, after the tabs).
    fn ribbon_rect(label: &str, last: bool) -> Option<Rect> {
        crate::ribbon::RECTS.with(|r| {
            let r = r.borrow();
            let mut it = r.iter().filter(|(l, _)| l == label).map(|(_, rect)| *rect);
            if last { it.next_back() } else { it.next() }
        })
    }

    fn node(&self, name: &str) -> Result<Node, String> {
        self.nodes
            .iter()
            .find(|n| n.name == name)
            .cloned()
            .ok_or_else(|| format!("no widget \"{name}\""))
    }

    /// The widget `name` in a window or a menu.
    fn in_window(&self, name: &str) -> Option<Node> {
        let windows = self.windows();
        let field = self.field_parts();
        self.nodes
            .iter()
            .find(|n| n.name == name && !field(n) && windows.iter().any(|w| w.contains_rect(n.rect)))
            .cloned()
    }

    /// A test for a text field or a part of its text: the name of such a
    /// widget is the text in the field, not a name to click.
    fn field_parts(&self) -> impl Fn(&Node) -> bool + use<> {
        let fields: Vec<Rect> = self
            .nodes
            .iter()
            .filter(|n| TEXT_FIELDS.contains(&n.role) || n.role == Role::SpinButton)
            .map(|n| n.rect.expand(1.0))
            .collect();
        move |n: &Node| fields.iter().any(|f| f.contains_rect(n.rect))
    }

    /// The widget `name` that is not a text field (the name of a text
    /// field is its text); of more than one, the one in a window or a
    /// menu. With none, a widget in a window whose name starts with `name`
    /// and a space (a row of a list): the row whose first part is `name`
    /// ("Varsity  ·  embroidery", not "Varsity Fill  ·  embroidery"),
    /// else the top row.
    fn button_node(&self, name: &str) -> Result<Node, String> {
        let windows = self.windows();
        let in_window = |n: &Node| windows.iter().any(|w| w.contains_rect(n.rect));
        let field = self.field_parts();
        let named: Vec<&Node> = self.nodes.iter().filter(|n| n.name == name && !field(n)).collect();
        let start = format!("{name} ");
        named
            .iter()
            .find(|n| in_window(n))
            .or(named.first())
            .copied()
            .or_else(|| {
                let first_part = |n: &Node| n.name.split("  ·  ").next() == Some(name);
                self.nodes
                    .iter()
                    .filter(|n| n.name.starts_with(&start) && in_window(n) && !field(n))
                    .min_by(|a, b| {
                        (!first_part(a))
                            .cmp(&!first_part(b))
                            .then(a.rect.top().total_cmp(&b.rect.top()))
                    })
            })
            .cloned()
            .ok_or_else(|| format!("no widget \"{name}\" (the windows have {:?})", self.window_names()))
    }

    /// The names of the widgets in the windows and menus (for a message).
    fn window_names(&self) -> Vec<&str> {
        let windows = self.windows();
        self.nodes
            .iter()
            .filter(|n| !n.name.is_empty() && windows.iter().any(|w| w.contains_rect(n.rect)))
            .map(|n| n.name.as_str())
            .take(60)
            .collect()
    }

    /// The text field with the hint text `hint` (in a window or a menu
    /// first), else the text field after the label `hint`.
    fn text_field(&self, hint: &str) -> Result<Node, String> {
        let windows = self.windows();
        let hinted: Vec<&Node> = self
            .nodes
            .iter()
            .filter(|n| TEXT_FIELDS.contains(&n.role) && n.placeholder == hint)
            .collect();
        hinted
            .iter()
            .find(|n| windows.iter().any(|w| w.contains_rect(n.rect)))
            .or(hinted.first())
            .map(|n| (*n).clone())
            .map_or_else(|| self.after_label(hint, &TEXT_FIELDS), Ok)
    }

    /// The widget of one of `roles` after the label `label`, on its row
    /// (a label in a window first).
    fn after_label(&self, label: &str, roles: &[Role]) -> Result<Node, String> {
        let l = self.button_node(label)?;
        let row: Vec<&Node> = self
            .nodes
            .iter()
            .filter(|n| roles.contains(&n.role))
            .filter(|n| (n.rect.center().y - l.rect.center().y).abs() < l.rect.height().max(12.0))
            .collect();
        let right = row
            .iter()
            .filter(|n| n.rect.left() >= l.rect.right() - 1.0)
            .min_by(|a, b| a.rect.left().total_cmp(&b.rect.left()));
        // A list made with `ComboBox::from_label` has its label after it.
        let left = || {
            row.iter()
                .filter(|n| n.rect.right() <= l.rect.left() + 1.0)
                .max_by(|a, b| a.rect.right().total_cmp(&b.rect.right()))
        };
        right
            .or_else(left)
            .map(|n| (*n).clone())
            .ok_or_else(|| format!("no field after \"{label}\""))
    }

    /// Sets the field after `label` to `value`: a click on it, all its
    /// text selected, the value typed, Enter.
    fn set_field(&mut self, label: &str, value: &str) -> Result<(), String> {
        let f = self.after_label(label, &[Role::SpinButton, Role::TextInput, Role::MultilineTextInput])?;
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
    /// not show the item, and has a search field, gets the item typed in it.
    fn choose(&mut self, label: &str, item: &str) -> Result<(), String> {
        let list = self.after_label(label, &[Role::ComboBox])?;
        self.click_at(list.rect.center(), Modifiers::NONE);
        if self.in_window(item).is_none()
            && let Ok(search) = self.text_field(tr("Search"))
        {
            self.click_select_all(&search);
            self.frame(vec![Event::Text(item.to_string())]);
            self.settle();
        }
        let n = self.in_window(item).ok_or_else(|| {
            format!(
                "no item \"{item}\" in the list \"{label}\" (the windows have {:?})",
                self.window_names()
            )
        })?;
        self.click_at(n.rect.center(), Modifiers::NONE);
        Ok(())
    }

    /// The close button of the window with the title `title`: on the row
    /// of the title, right of it.
    fn close_button(&self, title: &str) -> Result<Node, String> {
        let t = self.in_window(title).ok_or_else(|| format!("no window \"{title}\""))?;
        self.nodes
            .iter()
            .filter(|n| n.name == "Close window" && n.rect.left() > t.rect.left())
            .filter(|n| (n.rect.center().y - t.rect.center().y).abs() < t.rect.height())
            .min_by(|a, b| a.rect.left().total_cmp(&b.rect.left()))
            .cloned()
            .ok_or_else(|| format!("the window \"{title}\" has no close button"))
    }

    fn click_tab(&mut self, label: &str) -> Result<(), String> {
        let rect = Self::ribbon_rect(label, false).ok_or_else(|| format!("no tab \"{label}\""))?;
        self.click_at(rect.center(), Modifiers::NONE);
        Ok(())
    }

    /// Does one step.
    fn step(&mut self, s: &Step) -> Result<(), String> {
        let mods = holding(&s.holding)?;
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
                if Self::ribbon_rect(label, true).is_none() {
                    let folded = format!("{} {}", tr(panel), egui_phosphor::regular::CARET_DOWN);
                    let rect = Self::ribbon_rect(&folded, true).ok_or_else(|| format!("no button \"{label}\""))?;
                    self.click_at(rect.center(), Modifiers::NONE);
                }
                let rect = Self::ribbon_rect(label, true).ok_or_else(|| format!("no button \"{label}\""))?;
                self.click_at(rect.center(), Modifiers::NONE);
            }
        }
        if let Some(name) = &s.thread {
            let n = self.node(name)?;
            self.click_at(n.rect.center(), Modifiers::NONE);
        }
        if let Some([a, b]) = s.drag {
            let (from, to) = (self.screen(a)?, self.screen(b)?);
            self.drag(from, to, mods);
        }
        if let Some(p) = s.click {
            let at = self.screen(p)?;
            self.click_at(at, mods);
        }
        for c in &s.clicks {
            let (button, p) = match c.as_slice() {
                [x, y] => (PointerButton::Primary, [*x, *y]),
                [x, y, r] if *r == 1.0 => (PointerButton::Secondary, [*x, *y]),
                _ => return Err(format!("a click is [x, y] or [x, y, 1], not {c:?}")),
            };
            let at = self.screen(p)?;
            self.click_with(at, mods, button);
        }
        if let Some(k) = &s.key {
            let key = Key::from_name(k).ok_or_else(|| format!("no key \"{k}\""))?;
            self.key(key);
        }
        if let Some(text) = &s.button {
            let n = self.button_node(tr(text))?;
            if !n.enabled {
                return Err(format!("the button \"{text}\" is off"));
            }
            self.click_at(n.rect.center(), Modifiers::NONE);
        }
        if let Some(hint) = &s.field {
            let f = self.text_field(tr(hint))?;
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
            let n = self.close_button(tr(title))?;
            self.click_at(n.rect.center(), Modifiers::NONE);
            if self.in_window(tr(title)).is_some() {
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
        let st = self.app.state();
        let d = st.doc.design();
        let mut bad = Vec::new();
        if let Some(n) = e.objects
            && d.objects().len() != n
        {
            bad.push(format!("{} objects, not {n}", d.objects().len()));
        }
        if let Some(kinds) = &e.kinds {
            let now: Vec<&str> = d.objects().iter().map(|o| o.kind.type_name()).collect();
            if &now != kinds {
                bad.push(format!("the kinds are {now:?}, not {kinds:?}"));
            }
        }
        if let Some(threads) = &e.threads {
            let now: Vec<&str> = d
                .objects()
                .iter()
                .map(|o| d.thread(o.thread).map_or("", |t| t.name.as_str()))
                .collect();
            if &now != threads {
                bad.push(format!("the threads are {now:?}, not {threads:?}"));
            }
        }
        if let Some(n) = e.colors
            && d.sewn_color_blocks().len() != n
        {
            bad.push(format!("{} colors, not {n}", d.sewn_color_blocks().len()));
        }
        if let Some([w, h]) = e.size {
            let b = d.extents().ok_or("the design is empty")?;
            if (b.width() - w).abs() > 0.5 || (b.height() - h).abs() > 0.5 {
                bad.push(format!(
                    "the size is {:.1} × {:.1} mm, not {w} × {h}",
                    b.width(),
                    b.height()
                ));
            }
        }
        if let Some(n) = e.selected
            && st.selection.len() != n
        {
            bad.push(format!("{} selected, not {n}", st.selection.len()));
        }
        if let Some([lo, hi]) = e.stitches {
            let n = st.plan_view().plan.stitches.stitch_count();
            if !(lo..=hi).contains(&n) {
                bad.push(format!("{n} stitches, not {lo} to {hi}"));
            }
        }
        if let Some(n) = e.trims {
            let now = st.plan_view().plan.stitches.trim_count();
            if now != n {
                bad.push(format!("{now} trims, not {n}"));
            }
        }
        if let Some(place) = &e.on {
            let here: Vec<&str> = place.split(" > ").map(tr).collect();
            let (cmd, _, _) = command_at(&here)?;
            if !self.app.state().is_on(cmd) {
                bad.push(format!("\"{place}\" is not on"));
            }
        }
        let st = self.app.state();
        let d = st.doc.design();
        if let Some(id) = &e.fabric
            && d.fabric.as_deref() != Some(id.as_str())
        {
            bad.push(format!("the fabric is {:?}, not {id}", d.fabric));
        }
        if let Some(n) = e.holes {
            let now: usize = d
                .objects()
                .iter()
                .map(|o| match &o.kind {
                    emb_model::ObjectKind::Fill { holes, .. } => holes.len(),
                    _ => 0,
                })
                .sum();
            if now != n {
                bad.push(format!("{now} holes, not {n}"));
            }
        }
        if let Some(n) = e.hoopings {
            let now = d.hooping.as_ref().map_or(0, |h| h.hoopings.len());
            if now != n {
                bad.push(format!("{now} hoopings, not {n}"));
            }
        }
        if let Some(n) = e.names {
            let now = d.team_names.as_ref().map_or(0, |t| t.rows.len());
            if now != n {
                bad.push(format!("{now} rows of names, not {n}"));
            }
        }
        if let Some(n) = e.warnings {
            let info = st.design_info();
            let st = &*st;
            if info.issues.len() != n {
                let texts: Vec<String> = info
                    .issues
                    .iter()
                    .map(|i| {
                        let objects = st.doc.design().objects();
                        let at = i.object.and_then(|id| objects.iter().position(|o| o.id == id));
                        format!("{} (object {at:?})", i.text)
                    })
                    .collect();
                bad.push(format!("{} warnings, not {n}: {texts:?}", texts.len()));
            }
        }
        if let Some(name) = &e.shows {
            self.node(tr(name))?;
        }
        if let Some(title) = &e.window
            && self.in_window(tr(title)).is_none()
        {
            bad.push(format!("no window \"{title}\""));
        }
        if bad.is_empty() {
            Ok(())
        } else {
            let message = &self.app.state().message;
            if !message.is_empty() {
                bad.push(format!("the status line says \"{message}\""));
            }
            Err(bad.join("; "))
        }
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
                        let e = self.app.state().doc.design().extents().ok_or("the design is empty")?;
                        let e = e.expand(5.0);
                        [[e.min.x, e.min.y], [e.max.x, e.max.y]]
                    }
                };
                let (a, b) = (self.screen(a)?, self.screen(b)?);
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
                let r = Self::ribbon_rect(&key, true).ok_or_else(|| format!("no panel \"{title}\""))?;
                (r.expand(6.0), true)
            }
            "widgets" => {
                let mut all: Option<Rect> = None;
                for w in &s.widgets {
                    let r = self.node(tr(w))?.rect;
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
        self.studio.ctx.memory(|m| {
            let visible = m.areas().visible_layer_ids();
            m.layer_ids()
                .filter(|l| visible.contains(l) && matches!(l.order, egui::Order::Middle | egui::Order::Foreground))
                .filter_map(|l| m.area_rect(l.id))
                .collect()
        })
    }

    /// The items of the File menu (their texts, in the language of now).
    fn file_menu(&mut self) -> Result<Vec<String>, String> {
        let file = Self::ribbon_rect("File", false).ok_or("no File button")?;
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
    let steps: Steps = toml::from_str(text).map_err(|e| format!("{id}: {e}"))?;
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
