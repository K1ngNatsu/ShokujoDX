//! The step files of the tutorials (P19 Part 2, ADR 0162 and ADR 0177).
//!
//! - `docs/help/steps/<id>.toml` has the steps of the tutorial `<id>`: the
//!   clicks and keys of a user, the checks of the design, and the pictures
//!   of the topic. `build.rs` puts them in the program ([`HELP_STEPS`]).
//! - The lessons (`lesson.rs`) do them with the user in the program: a
//!   step with [`Step::item`] starts a stage of the lesson at that item of
//!   the topic. The tests of the tutorials (`tutorial_steps.rs`) do them
//!   with pointer and key events.
//! - A step names a command by its place in English (`Digitize > Shapes >
//!   Ellipse`) and a button by its English text; the lesson and the tests
//!   find them in the language of now with `tr`, through the screen
//!   reader names of the widgets ([`Screen`]).

use eframe::egui::accesskit::{Role, TreeUpdate};
use eframe::egui::{self, Pos2, Rect};
use serde::Deserialize;

use crate::app::AppState;
use crate::i18n::tr;
use crate::ribbon::{Cmd, RibbonTab, all_commands};

// The step files: `HELP_STEPS`, (tutorial id, text), made by `build.rs`.
include!(concat!(env!("OUT_DIR"), "/help_steps.rs"));

/// The text of the step file of the tutorial `id`, if it has one.
pub fn file(id: &str) -> Option<&'static str> {
    HELP_STEPS.iter().find(|(i, _)| *i == id).map(|(_, t)| *t)
}

/// The roles of text fields: the name of such a widget is its text.
pub const TEXT_FIELDS: [Role; 3] = [Role::TextInput, Role::MultilineTextInput, Role::SearchInput];

/// The steps of one tutorial.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Steps {
    pub step: Vec<Step>,
}

/// Reads a step file.
pub fn parse(text: &str) -> Result<Steps, String> {
    toml::from_str(text).map_err(|e| e.to_string())
}

/// One step: one of the actions, a check, or a picture.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Step {
    /// The lesson: a stage starts at this step, and shows this numbered
    /// item of the topic (the English `##` heading of its section and the
    /// number of the item; 0: the whole section) and the items after it,
    /// up to the item of the next stage.
    pub item: Option<(String, u32)>,
    /// The tests need this step, but the topic does not ask the user to
    /// do it (an Esc that closes a menu): the lesson does not wait for it.
    pub test_only: bool,
    /// Open the start design of this name (as its link in the topic).
    pub open: Option<String>,
    /// Click a tab of the ribbon (its English name).
    pub tab: Option<String>,
    /// Click a command: `Tab > Panel > Command`, `Tab > Command`, or the
    /// name of a command next to the File button.
    pub command: Option<String>,
    /// The command opens a window of the system (a file or a folder): the
    /// test only checks that it is there and can run.
    pub check_only: bool,
    /// Click the color of the thread of this name in the Threads strip.
    pub thread: Option<String>,
    /// Drag on the canvas from the first point to the second (mm).
    pub drag: Option<[[f64; 2]; 2]>,
    /// Click on the canvas at this point (mm).
    pub click: Option<[f64; 2]>,
    /// Clicks on the canvas, one after the other (mm): `[x, y]` with the
    /// left button (a corner), `[x, y, 1]` with the right button (a curve
    /// point).
    pub clicks: Vec<Vec<f64>>,
    /// The keys held for `drag` and `click`: `ctrl`, `shift`, `alt`.
    pub holding: Vec<String>,
    /// Push a key: `Enter`, `Escape`, `Delete`.
    pub key: Option<String>,
    /// Click the button with this English text (in a window or a menu
    /// if more than one widget has the name). With no widget of that name,
    /// a widget in a window whose name starts with the text and a space
    /// (a row of a list: "Graduate Regular  ·  TrueType").
    pub button: Option<String>,
    /// Click the text field with this English hint text (a search field,
    /// a field with no label), or the text field after this English label.
    /// Then `type` types into it.
    pub field: Option<String>,
    /// Add this font file (a path from the folder of the repository) to the
    /// fonts, as a font of the computer: the step files do not depend on
    /// the fonts of the computer that runs them.
    pub font_file: Option<String>,
    /// Close the window with this English title.
    pub close: Option<String>,
    /// Type this text (into the field that has the keyboard).
    #[serde(rename = "type")]
    pub text: Option<String>,
    /// Set the field after the English label to the value: `["Height",
    /// "25"]` (a number field or a text field on the row of the label).
    pub set: Option<[String; 2]>,
    /// Choose an item of the list after the English label: `["Baseline",
    /// "Arc over"]` (the item in English too; or the start of the item
    /// before a space, as `button`).
    pub choose: Option<[String; 2]>,
    /// Check the design and the program.
    pub expect: Option<Expect>,
    /// Make the picture of this name.
    pub picture: Option<String>,
    /// The part of the screen in the picture: `canvas` (with `around`),
    /// `windows` (the windows that are open), `panel` (a panel of the
    /// ribbon, with `panel`) or `widgets` (with `widgets`).
    pub area: Option<String>,
    /// The English title of the ribbon panel in a picture of a panel.
    pub panel: Option<String>,
    /// The part of the design in a picture of the canvas (mm, two corners);
    /// with none, the whole design (with the product that shows) and 5 mm
    /// round it.
    pub around: Option<[[f64; 2]; 2]>,
    /// The widgets in a picture of the area `widgets` (English names).
    pub widgets: Vec<String>,
}

impl Step {
    /// The step does something that a user does (not only a check, a
    /// picture or a font for the test).
    pub fn acts(&self) -> bool {
        self.open.is_some()
            || self.tab.is_some()
            || self.command.is_some()
            || self.thread.is_some()
            || self.drag.is_some()
            || self.click.is_some()
            || !self.clicks.is_empty()
            || self.key.is_some()
            || self.button.is_some()
            || self.field.is_some()
            || self.close.is_some()
            || self.text.is_some()
            || self.set.is_some()
            || self.choose.is_some()
    }
}

/// What the design and the program must be after the steps before.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Expect {
    /// The number of objects.
    pub objects: Option<usize>,
    /// The kinds of the objects in sew order (`ObjectKind::type_name`).
    pub kinds: Option<Vec<String>>,
    /// The names of the threads of the objects in sew order.
    pub threads: Option<Vec<String>>,
    /// The number of colors that sew.
    pub colors: Option<usize>,
    /// The number of stitches is in this range.
    pub stitches: Option<[usize; 2]>,
    /// The number of trims (the status line).
    pub trims: Option<usize>,
    /// The width and the height of the design (mm, within 0.5 mm).
    pub size: Option<[f64; 2]>,
    /// The number of selected objects.
    pub selected: Option<usize>,
    /// A widget with this English name shows (a button, a label).
    pub shows: Option<String>,
    /// A window or a menu with this English title shows.
    pub window: Option<String>,
    /// The command at this place is on (a view, a tool).
    pub on: Option<String>,
    /// The id of the fabric of the design.
    pub fabric: Option<String>,
    /// The number of warnings and problems of Design Info.
    pub warnings: Option<usize>,
    /// The number of holes of all fills.
    pub holes: Option<usize>,
    /// The number of hoopings (Hooping mode).
    pub hoopings: Option<usize>,
    /// The number of rows of the team names.
    pub names: Option<usize>,
    /// The number of threads in the thread stock.
    pub stock: Option<usize>,
    /// The product under the design (its English name, `ProductKind::name`)
    /// and the place of the design on it.
    pub product: Option<[String; 2]>,
    /// Sketch mode is open (or not).
    pub sketch: Option<bool>,
    /// The number of closed regions of the open sketch.
    pub regions: Option<usize>,
    /// The spacing of each satin column (mm, within 0.001 mm).
    pub spacing: Option<f64>,
    /// The id of the hoop of the design (`data/hoops.toml`).
    pub hoop: Option<String>,
}

/// A check of [`Expect`] that does not agree.
#[derive(Debug, Clone, PartialEq)]
pub enum Miss {
    /// The number of objects now, and the number of the step.
    Objects(usize, usize),
    /// The number of selected objects now, and the number of the step.
    Selected(usize, usize),
    /// The English title of a window that does not show.
    Window(String),
    /// Another check (an English text for the tests).
    Other(String),
}

impl Miss {
    /// The text for the tests (English).
    #[cfg(test)]
    pub fn english(&self) -> String {
        match self {
            Miss::Objects(now, n) => format!("{now} objects, not {n}"),
            Miss::Selected(now, n) => format!("{now} selected, not {n}"),
            Miss::Window(title) => format!("no window \"{title}\""),
            Miss::Other(t) => t.clone(),
        }
    }

    /// The text for the user (in the language of now).
    pub fn text(&self) -> String {
        match self {
            Miss::Objects(now, n) => {
                crate::i18n::fill("The design has {} objects; after this step it has {}.", &[now, n])
            }
            Miss::Selected(now, n) => crate::i18n::fill("{} objects are selected; this step needs {}.", &[now, n]),
            Miss::Window(title) => crate::i18n::fill("The window \"{}\" is not open.", &[&tr(title)]),
            Miss::Other(_) => tr("The design is not yet as this step makes it.").to_string(),
        }
    }
}

impl Expect {
    /// The checks that do not agree. `exact`: all checks (the tests);
    /// else not the numbers that depend on the machine, the hoop and the
    /// hand of the user (stitches, trims, the size and the warnings): a
    /// lesson checks what each step makes.
    pub fn misses(&self, st: &mut AppState, screen: &Screen, exact: bool) -> Vec<Miss> {
        let mut bad = Vec::new();
        let mut other = |t: String| bad.push(Miss::Other(t));
        let d = st.doc.design();
        if let Some(kinds) = &self.kinds {
            let now: Vec<&str> = d.objects().iter().map(|o| o.kind.type_name()).collect();
            if &now != kinds {
                other(format!("the kinds are {now:?}, not {kinds:?}"));
            }
        }
        if let Some(threads) = &self.threads {
            let now: Vec<&str> = d
                .objects()
                .iter()
                .map(|o| d.thread(o.thread).map_or("", |t| t.name.as_str()))
                .collect();
            if &now != threads {
                other(format!("the threads are {now:?}, not {threads:?}"));
            }
        }
        if let Some(n) = self.colors
            && d.sewn_color_blocks().len() != n
        {
            other(format!("{} colors, not {n}", d.sewn_color_blocks().len()));
        }
        if exact && let Some([w, h]) = self.size {
            match d.extents() {
                None => other("the design is empty".into()),
                Some(b) if (b.width() - w).abs() > 0.5 || (b.height() - h).abs() > 0.5 => other(format!(
                    "the size is {:.1} × {:.1} mm, not {w} × {h}",
                    b.width(),
                    b.height()
                )),
                Some(_) => {}
            }
        }
        if let Some(id) = &self.fabric
            && d.fabric.as_deref() != Some(id.as_str())
        {
            other(format!("the fabric is {:?}, not {id}", d.fabric));
        }
        if let Some(n) = self.holes {
            let now: usize = d
                .objects()
                .iter()
                .map(|o| match &o.kind {
                    emb_model::ObjectKind::Fill { holes, .. } => holes.len(),
                    _ => 0,
                })
                .sum();
            if now != n {
                other(format!("{now} holes, not {n}"));
            }
        }
        if let Some(n) = self.hoopings {
            let now = d.hooping.as_ref().map_or(0, |h| h.hoopings.len());
            if now != n {
                other(format!("{now} hoopings, not {n}"));
            }
        }
        if let Some(n) = self.names {
            let now = d.team_names.as_ref().map_or(0, |t| t.rows.len());
            if now != n {
                other(format!("{now} rows of names, not {n}"));
            }
        }
        if let Some(id) = &self.hoop {
            let now = match &d.hoop.selection {
                emb_model::HoopSelection::Preset { id } => id.as_str(),
                emb_model::HoopSelection::OpenField => "open field",
            };
            if now != id {
                other(format!("the hoop is {now}, not {id}"));
            }
        }
        if let Some(want) = self.spacing {
            let now: Vec<f64> = d
                .objects()
                .iter()
                .filter_map(|o| match &o.kind {
                    emb_model::ObjectKind::Satin { params, .. } => Some(params.spacing_mm),
                    _ => None,
                })
                .collect();
            if now.is_empty() || now.iter().any(|v| (v - want).abs() > 0.001) {
                other(format!("the satin spacings are {now:?}, not {want}"));
            }
        }
        if let Some([kind, place]) = &self.product {
            let now = d
                .product
                .as_ref()
                .map(|v| (v.kind.name(), v.placement_of(d).map(|p| p.name)));
            if now != Some((kind.as_str(), Some(place.as_str()))) {
                other(format!("the product is {now:?}, not {kind} at {place}"));
            }
        }
        if let Some(open) = self.sketch
            && st.sketch.is_some() != open
        {
            other(format!("sketch mode is {}", if open { "not open" } else { "open" }));
        }
        if let Some(n) = self.stock {
            let now = st.settings.thread_stock.threads.len();
            if now != n {
                other(format!("{now} threads in stock, not {n}"));
            }
        }
        if let Some(place) = &self.on {
            let here: Vec<&str> = place.split(" > ").map(tr).collect();
            match command_at(&here) {
                Ok((cmd, _, _)) if st.is_on(cmd) => {}
                Ok(_) => other(format!("\"{place}\" is not on")),
                Err(e) => other(e),
            }
        }
        if let Some(name) = &self.shows
            && let Err(e) = screen.node(tr(name))
        {
            other(e);
        }
        if let Some(n) = self.regions {
            let now = st.sketch_regions().len();
            if now != n {
                other(format!("{now} regions, not {n}"));
            }
        }
        if exact && let Some([lo, hi]) = self.stitches {
            let n = st.plan_view().plan.stitches.stitch_count();
            if !(lo..=hi).contains(&n) {
                other(format!("{n} stitches, not {lo} to {hi}"));
            }
        }
        if exact && let Some(n) = self.trims {
            let now = st.plan_view().plan.stitches.trim_count();
            if now != n {
                other(format!("{now} trims, not {n}"));
            }
        }
        if exact && let Some(n) = self.warnings {
            let info = st.design_info();
            if info.issues.len() != n {
                let objects = st.doc.design().objects();
                let texts: Vec<String> = info
                    .issues
                    .iter()
                    .map(|i| {
                        let at = i.object.and_then(|id| objects.iter().position(|o| o.id == id));
                        let place =
                            i.at.map(|p| format!(" at ({:.1}, {:.1})", p.x, p.y))
                                .unwrap_or_default();
                        format!("{} (object {at:?}){place}", i.text)
                    })
                    .collect();
                other(format!("{} warnings, not {n}: {texts:?}", texts.len()));
            }
        }
        // The numbers that a user sees first go first.
        let d = st.doc.design();
        if let Some(n) = self.selected
            && st.selection.len() != n
        {
            bad.insert(0, Miss::Selected(st.selection.len(), n));
        }
        if let Some(n) = self.objects
            && d.objects().len() != n
        {
            bad.insert(0, Miss::Objects(d.objects().len(), n));
        }
        if let Some(title) = &self.window
            && screen.in_window(tr(title)).is_none()
        {
            bad.insert(0, Miss::Window(title.clone()));
        }
        bad
    }
}

/// The command at `place` (in the language of now): `Tab > Panel >
/// Command`, `Tab > Command`, or the name of a command next to the File
/// button. Also its tab and the English title of its panel.
pub fn command_at(place: &[&str]) -> Result<(Cmd, RibbonTab, &'static str), String> {
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

/// A widget for a screen reader: its name (empty if it has none), its
/// place and its kind.
#[derive(Debug, Clone)]
pub struct Node {
    /// The id of the node (the value of the egui id of the widget).
    pub id: u64,
    pub name: String,
    /// The text of a field, or the item that a list shows.
    pub value: String,
    /// The number of a number field.
    pub number: Option<f64>,
    /// The hint text of a text field.
    pub placeholder: String,
    pub rect: Rect,
    pub enabled: bool,
    pub role: Role,
}

/// The widgets of a screen reader tree of egui (not the runs of text in
/// a label or a field: they have the text of the label as their name).
pub fn nodes_of(update: &TreeUpdate) -> Vec<Node> {
    update
        .nodes
        .iter()
        .filter(|(_, n)| n.role() != Role::TextRun)
        .filter_map(|(id, n)| {
            let b = n.bounds()?;
            Some(Node {
                id: id.0,
                name: n.label().or(n.value()).unwrap_or_default().to_string(),
                value: n.value().unwrap_or_default().to_string(),
                number: n.numeric_value(),
                placeholder: n.placeholder().unwrap_or_default().to_string(),
                rect: Rect::from_min_max(Pos2::new(b.x0 as f32, b.y0 as f32), Pos2::new(b.x1 as f32, b.y1 as f32)),
                enabled: !n.is_disabled(),
                role: n.role(),
            })
        })
        .collect()
}

/// The windows and menus that show (in points).
pub fn windows(ctx: &egui::Context) -> Vec<Rect> {
    ctx.memory(|m| {
        let visible = m.areas().visible_layer_ids();
        m.layer_ids()
            .filter(|l| visible.contains(l) && matches!(l.order, egui::Order::Middle | egui::Order::Foreground))
            .filter_map(|l| m.area_rect(l.id))
            .collect()
    })
}

/// The widgets of a frame and the windows that show: the lesson and the
/// tests find the widgets of a step in it.
#[derive(Debug, Clone, Default)]
pub struct Screen {
    pub nodes: Vec<Node>,
    pub windows: Vec<Rect>,
}

impl Screen {
    /// The screen with no widgets of the windows `ids` that show (the
    /// lesson and the Help window: their texts name the buttons that a
    /// step clicks).
    pub fn without(mut self, ctx: &egui::Context, ids: &[egui::Id]) -> Screen {
        let rects: Vec<Rect> = ctx.memory(|m| {
            let visible = m.areas().visible_layer_ids();
            ids.iter()
                .filter(|id| visible.contains(&egui::LayerId::new(egui::Order::Middle, **id)))
                .filter_map(|id| m.area_rect(*id))
                .collect()
        });
        // The widgets of those windows, by their layer (a window that
        // changes its height has the rect of the frame before).
        let layers: Vec<egui::LayerId> = ids
            .iter()
            .map(|id| egui::LayerId::new(egui::Order::Middle, *id))
            .collect();
        let quiet: std::collections::HashSet<u64> = ctx.viewport(|v| {
            layers
                .iter()
                .flat_map(|l| {
                    v.prev_pass
                        .widgets
                        .get_layer(*l)
                        .chain(v.this_pass.widgets.get_layer(*l))
                })
                .map(|w| w.id.value())
                .collect()
        });
        // A widget in another window (under the Lesson or Help window)
        // stays: a step can name it (the audit of 2026-11-26).
        let others: Vec<Rect> = self.windows.iter().filter(|w| !rects.contains(w)).copied().collect();
        self.nodes.retain(|n| {
            !quiet.contains(&n.id)
                && (others.iter().any(|w| w.contains_rect(n.rect)) || !rects.iter().any(|r| r.contains_rect(n.rect)))
        });
        self.windows.retain(|w| !rects.contains(w));
        self
    }

    pub fn node(&self, name: &str) -> Result<Node, String> {
        self.nodes
            .iter()
            .find(|n| n.name == name)
            .cloned()
            .ok_or_else(|| format!("no widget \"{name}\""))
    }

    fn in_any_window(&self, r: Rect) -> bool {
        self.windows.iter().any(|w| w.contains_rect(r))
    }

    /// The widget `name` in a window or a menu.
    pub fn in_window(&self, name: &str) -> Option<Node> {
        let field = self.field_parts();
        self.nodes
            .iter()
            .find(|n| n.name == name && !field(n) && self.in_any_window(n.rect))
            .cloned()
    }

    /// The item `name` of an open list: the item of that name, else the
    /// item whose name starts with `name` and a space ("Same Density for
    /// All Satins (sample)", where the end is in the language of now).
    pub fn list_item(&self, name: &str) -> Option<Node> {
        self.in_window(name).or_else(|| {
            let field = self.field_parts();
            let start = format!("{name} ");
            self.nodes
                .iter()
                .find(|n| n.name.starts_with(&start) && !field(n) && self.in_any_window(n.rect))
                .cloned()
        })
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
    pub fn button_node(&self, name: &str) -> Result<Node, String> {
        let field = self.field_parts();
        let named: Vec<&Node> = self.nodes.iter().filter(|n| n.name == name && !field(n)).collect();
        let start = format!("{name} ");
        named
            .iter()
            .find(|n| self.in_any_window(n.rect))
            .or(named.first())
            .copied()
            .or_else(|| {
                let first_part = |n: &Node| n.name.split("  ·  ").next() == Some(name);
                self.nodes
                    .iter()
                    .filter(|n| n.name.starts_with(&start) && self.in_any_window(n.rect) && !field(n))
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
    pub fn window_names(&self) -> Vec<&str> {
        self.nodes
            .iter()
            .filter(|n| !n.name.is_empty() && self.in_any_window(n.rect))
            .map(|n| n.name.as_str())
            .take(60)
            .collect()
    }

    /// The text field with the hint text `hint` (in a window or a menu
    /// first), else the text field after the label `hint`.
    pub fn text_field(&self, hint: &str) -> Result<Node, String> {
        let hinted: Vec<&Node> = self
            .nodes
            .iter()
            .filter(|n| TEXT_FIELDS.contains(&n.role) && n.placeholder == hint)
            .collect();
        hinted
            .iter()
            .find(|n| self.in_any_window(n.rect))
            .or(hinted.first())
            .map(|n| (*n).clone())
            .map_or_else(|| self.after_label(hint, &TEXT_FIELDS), Ok)
    }

    /// The widget of one of `roles` after the label `label`, on its row
    /// (a label in a window first).
    pub fn after_label(&self, label: &str, roles: &[Role]) -> Result<Node, String> {
        // A label of that name with such a field on its row (a window can
        // have the name two times: "Quantity" of a field and of a column);
        // in a window first.
        let field = self.field_parts();
        let mut labels: Vec<Node> = self
            .nodes
            .iter()
            .filter(|n| n.name == label && !field(n))
            .cloned()
            .collect();
        labels.sort_by_key(|n| !self.in_any_window(n.rect));
        if labels.is_empty() {
            labels.push(self.button_node(label)?);
        }
        labels
            .iter()
            .find_map(|l| self.field_after(l, roles))
            .ok_or_else(|| format!("no field after \"{label}\""))
    }

    /// The widget with one of `roles` right of the label `l` on its row, or
    /// left of it (a list made with `ComboBox::from_label` has its label
    /// after it).
    fn field_after(&self, l: &Node, roles: &[Role]) -> Option<Node> {
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
        let left = || {
            row.iter()
                .filter(|n| n.rect.right() <= l.rect.left() + 1.0)
                .max_by(|a, b| a.rect.right().total_cmp(&b.rect.right()))
        };
        right.or_else(left).map(|n| (*n).clone())
    }

    /// The close button of the window with the title `title`: on the row
    /// of the title, right of it.
    /// A label in the window can have the text of the title (in Japanese
    /// "Family" and "Families" are both ファミリー): the title is the one
    /// with a close button on its row.
    pub fn close_button(&self, title: &str) -> Result<Node, String> {
        self.in_window(title).ok_or_else(|| format!("no window \"{title}\""))?;
        self.nodes
            .iter()
            .filter(|t| t.name == title && self.in_any_window(t.rect))
            .find_map(|t| {
                self.nodes
                    .iter()
                    .filter(|n| n.name == "Close window" && n.rect.left() > t.rect.left())
                    .filter(|n| (n.rect.center().y - t.rect.center().y).abs() < t.rect.height())
                    .min_by(|a, b| a.rect.left().total_cmp(&b.rect.left()))
            })
            .cloned()
            .ok_or_else(|| format!("the window \"{title}\" has no close button"))
    }
}
