//! Help (P10 Milestone 4): the help topics, the search, F1 and the Help
//! window.
//!
//! - The topics are Markdown files in `docs/help`, built into the program.
//!   Each file starts with a head between two `---` lines: `id`, `title`,
//!   `group` (start, tutorials, design, production, specialty, reference)
//!   and `keywords`.
//! - The topics of the ribbon tabs and of the keyboard are made from the
//!   ribbon (the name, the tooltip and the shortcut of each command), so
//!   they always agree with the program.
//! - The Markdown is a small part of Markdown: headings (`#`, `##`, `###`),
//!   paragraphs, lists (`-` and `1.`), tables (`| a | b |`, the first row
//!   is the head), `**bold**`, `` `code` `` and links `[text](topic-id)`.
//! - The search answers questions from the help text, with no network: it
//!   ranks the sections (BM25) by the words of the question, with the
//!   question words left out, each word reduced to its stem, and words of
//!   the same meaning added. The best section gives the answer: its
//!   sentence or list item with most words of the question.
//! - F1 opens the help of the ribbon command under the pointer, of the
//!   window under the pointer ([`WINDOW_HELP`]), or of the active tool,
//!   else the Getting started topic (ADR 0055).

use std::collections::{HashMap, HashSet};

use eframe::egui::{self, RichText};

use crate::app::AppState;
use crate::draw::Tool;
use crate::i18n::tr;
use crate::ribbon::{Cmd, RibbonTab};

/// The id of the keyboard shortcuts topic.
pub const KEYBOARD_ID: &str = "keyboard";

/// The help files, in the order of the contents (in each group).
const FILES: &[&str] = &[
    include_str!("../../../docs/help/getting-started.md"),
    include_str!("../../../docs/help/embroidery-basics.md"),
    include_str!("../../../docs/help/troubleshooting.md"),
    include_str!("../../../docs/help/tutorial-patch.md"),
    include_str!("../../../docs/help/tutorial-lettering.md"),
    include_str!("../../../docs/help/tutorial-applique.md"),
    include_str!("../../../docs/help/tutorial-towel.md"),
    include_str!("../../../docs/help/tutorial-hooping.md"),
    include_str!("../../../docs/help/tutorial-lace.md"),
    include_str!("../../../docs/help/tutorial-output.md"),
    include_str!("../../../docs/help/canvas.md"),
    include_str!("../../../docs/help/digitizing.md"),
    include_str!("../../../docs/help/editing.md"),
    include_str!("../../../docs/help/arranging.md"),
    include_str!("../../../docs/help/objects-and-stitches.md"),
    include_str!("../../../docs/help/stitch-types.md"),
    include_str!("../../../docs/help/lettering.md"),
    include_str!("../../../docs/help/pictures.md"),
    include_str!("../../../docs/help/artwork.md"),
    include_str!("../../../docs/help/sketch.md"),
    include_str!("../../../docs/help/colors-and-threads.md"),
    include_str!("../../../docs/help/fabrics-and-styles.md"),
    include_str!("../../../docs/help/machines-and-hoops.md"),
    include_str!("../../../docs/help/hooping.md"),
    include_str!("../../../docs/help/checking.md"),
    include_str!("../../../docs/help/files-and-output.md"),
    include_str!("../../../docs/help/quotes.md"),
    include_str!("../../../docs/help/thread-stock.md"),
    include_str!("../../../docs/help/scripts.md"),
    include_str!("../../../docs/help/library.md"),
    include_str!("../../../docs/help/specialty.md"),
    include_str!("../../../docs/help/cross-stitch.md"),
    include_str!("../../../docs/help/rhinestones.md"),
    include_str!("../../../docs/help/quilting.md"),
    include_str!("../../../docs/help/cutwork.md"),
    include_str!("../../../docs/help/options.md"),
    include_str!("../../../docs/help/file-formats.md"),
    include_str!("../../../docs/help/glossary.md"),
];

/// The part of the contents that a topic is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Group {
    Start,
    Tutorials,
    /// Drawing, changing and arranging objects, stitches, lettering,
    /// pictures and artwork.
    Designing,
    /// Threads, fabrics, machines, hoops, checks, files and the library.
    Production,
    Specialty,
    Ribbon,
    Reference,
}

impl Group {
    pub const ALL: [Group; 7] = [
        Group::Start,
        Group::Tutorials,
        Group::Designing,
        Group::Production,
        Group::Specialty,
        Group::Ribbon,
        Group::Reference,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Group::Start => tr("Start"),
            Group::Tutorials => tr("Tutorials"),
            Group::Designing => tr("Designing"),
            Group::Production => tr("Production"),
            Group::Specialty => tr("Specialty"),
            Group::Ribbon => tr("The ribbon"),
            Group::Reference => tr("Reference"),
        }
    }

    fn parse(s: &str) -> Group {
        match s {
            "start" => Group::Start,
            "tutorials" => Group::Tutorials,
            "production" => Group::Production,
            "specialty" => Group::Specialty,
            "reference" => Group::Reference,
            _ => Group::Designing,
        }
    }
}

/// The help topic of each window, for F1: the id of the window (its English
/// title, or the id that the window sets) and the topic id.
pub const WINDOW_HELP: &[(&str, &str)] = &[
    ("Picture to Design", "pictures"),
    ("Lettering", "lettering"),
    ("Monogram", "lettering"),
    ("Team Names", "lettering"),
    ("Keyboard Designs", "lettering"),
    ("Fonts", "lettering"),
    ("Alphabets", "lettering"),
    ("Offsets", "artwork"),
    ("Border Trace", "artwork"),
    ("Arrangements", "arranging"),
    ("Paste Special", "editing"),
    ("Simplify", "editing"),
    ("Corner Radius", "editing"),
    ("Hole Inside the Outline", "editing"),
    ("Save as Clipart", "library"),
    ("Motifs", "stitch-types"),
    ("Borders", "stitch-types"),
    ("Buttonholes", "stitch-types"),
    ("Blends", "stitch-types"),
    ("ColorFills", "stitch-types"),
    ("Quilting", "quilting"),
    ("Cutwork", "cutwork"),
    ("Fabric", "fabrics-and-styles"),
    ("Template", "fabrics-and-styles"),
    ("My Assets", "fabrics-and-styles"),
    ("Machine Settings", "machines-and-hoops"),
    ("Needles", "machines-and-hoops"),
    ("Quote", "quotes"),
    ("Order Details", "quotes"),
    ("Thread Stock", "thread-stock"),
    ("Scripts", "scripts"),
    ("My Hoops", "machines-and-hoops"),
    ("Hoop Options", "machines-and-hoops"),
    ("Preview Hoopings", "hooping"),
    ("Product", "checking"),
    ("Repeats", "checking"),
    ("Overview", "canvas"),
    ("Output Design", "files-and-output"),
    ("Print Preview", "files-and-output"),
    ("Send to USB", "files-and-output"),
    ("Recover Designs", "files-and-output"),
    ("Options", "options"),
    ("Find a Command", "getting-started"),
];

/// The help topic of the window under the pointer, if it has one.
pub fn window_topic(ctx: &egui::Context) -> Option<&'static str> {
    let pos = ctx.pointer_latest_pos()?;
    let layer = ctx.layer_id_at(pos)?;
    WINDOW_HELP
        .iter()
        .find(|(name, _)| layer.id == egui::Id::new(*name) || layer.id == egui::Id::new(tr(name)))
        .map(|(_, topic)| *topic)
}

/// One help topic.
#[derive(Debug, Clone, PartialEq)]
pub struct Topic {
    pub id: String,
    pub title: String,
    pub group: Group,
    pub keywords: Vec<String>,
    /// Markdown.
    pub body: String,
}

/// Reads a help file: the head and the Markdown body.
pub fn parse_topic(text: &str) -> Option<Topic> {
    let text = text.replace("\r\n", "\n");
    let rest = text.strip_prefix("---\n")?;
    let end = rest.find("\n---\n")?;
    let (head, body) = (&rest[..end], &rest[end + 5..]);
    let mut fields: HashMap<&str, &str> = HashMap::new();
    for line in head.lines() {
        if let Some((k, v)) = line.split_once(':') {
            fields.insert(k.trim(), v.trim());
        }
    }
    let unquote = |s: &str| s.trim_matches('"').to_string();
    Some(Topic {
        id: fields.get("id")?.to_string(),
        title: unquote(fields.get("title")?),
        group: Group::parse(fields.get("group").copied().unwrap_or("guide")),
        keywords: fields
            .get("keywords")
            .map(|k| {
                k.split(',')
                    .map(|w| w.trim().to_string())
                    .filter(|w| !w.is_empty())
                    .collect()
            })
            .unwrap_or_default(),
        body: body.trim().to_string(),
    })
}

/// The id of the help topic of a ribbon tab.
pub fn ribbon_id(tab: RibbonTab) -> String {
    format!("ribbon-{}", tab.label().to_lowercase())
}

/// What each ribbon tab is for (the first lines of its topic).
fn tab_intro(tab: RibbonTab) -> &'static str {
    match tab {
        RibbonTab::Digitize => {
            "The Digitize tab has the tools that select objects and draw lines, fills, satin columns, shapes and              appliqué, and the decorative borders, buttonholes and blends. See [Drawing objects](digitizing) and              [Tutorial: an appliqué](tutorial-applique)."
        }
        RibbonTab::Lettering => {
            "The Lettering tab adds text, monograms, team names and keyboard designs, and opens the lettering              kiosk. See [Lettering and monograms](lettering) and [Tutorial: a name on an arc](tutorial-lettering)."
        }
        RibbonTab::Artwork => {
            "The Artwork tab imports pictures and vector artwork, traces pictures, draws and converts vectors, and              changes shapes (shaping, border traces and offsets). See [Pictures and Picture to design](pictures)              and [Vector artwork and shaping](artwork)."
        }
        RibbonTab::Specialty => {
            "The Specialty tab has cross stitch, rhinestones, cutwork, quilting, puff (3D foam) and laydown. See              [Specialty decoration](specialty)."
        }
        RibbonTab::Edit => {
            "The Edit tab copies and pastes, selects, splits and joins objects, changes shapes, lines and stitch              types, and sets machine functions. See [Changing objects](editing) and [Stitch types](stitch-types)."
        }
        RibbonTab::Arrange => {
            "The Arrange tab turns, mirrors, aligns, spaces and sizes objects, makes copies (arrangements and              mirror copies), groups and locks objects, and changes the sew order. See [Arranging and the sew              order](arranging)."
        }
        RibbonTab::Color => {
            "The Color tab has the threads, the thread charts, the colorways and the fabric color. See [Colors              and threads](colors-and-threads)."
        }
        RibbonTab::View => {
            "The View tab shows and hides parts of the design, has the stitch player, the stitch cursor and the              filters, the zoom and the workspace settings. See [The canvas and the selection](canvas) and              [Checking a design](checking)."
        }
        RibbonTab::Design => {
            "The Design tab sets the machine, the hoop, the fabric and the styles, has the design library, the              templates, the repeats and the product, and makes the output files. See [Machines and              hoops](machines-and-hoops) and [Files and output](files-and-output)."
        }
        RibbonTab::Hooping => {
            "The Hooping tab splits a design that is larger than the hoop into hoopings. See [Designs larger than              the hoop](hooping)."
        }
        RibbonTab::Sketch => {
            "The Sketch tab shows while a sketch is open. It draws lines, arcs, circles, rectangles and polygons,              adds constraints and dimensions, and finishes the sketch. See [Sketches](sketch)."
        }
        RibbonTab::Help => "The Help tab has the help topics, the tutorials, the start page and simple mode.",
    }
}

/// The topic of a ribbon tab, made from its panels and commands.
fn ribbon_topic(tab: RibbonTab, ctx: &egui::Context) -> Topic {
    let mut body = format!("# {} tab\n\n{}\n", tab.label(), tab_intro(tab));
    for p in crate::ribbon::help_panels(tab, ctx) {
        body.push_str(&format!("\n## {}\n\n", p.title));
        if p.commands.is_empty() {
            body.push_str("Put the pointer on the lists and fields of this panel to see what they do.\n");
        }
        for c in p.commands {
            let keys = c.keys.map(|k| format!(" ({k})")).unwrap_or_default();
            body.push_str(&format!("- **{}**{keys}: {}\n", c.label, c.tip));
        }
    }
    Topic {
        id: ribbon_id(tab),
        title: format!("{} tab", tab.label()),
        group: Group::Ribbon,
        keywords: vec![tab.label().to_lowercase(), "ribbon".into(), "tab".into()],
        body,
    }
}

/// The keyboard shortcuts topic: the shortcuts of the ribbon commands and
/// the keys of the canvas, the tools and the stitch cursor.
fn keyboard_topic(ctx: &egui::Context) -> Topic {
    let mut body = String::from("# Keyboard shortcuts\n\n## Commands\n\n");
    let mut seen = HashSet::new();
    for tab in RibbonTab::ALL {
        for p in crate::ribbon::help_panels(tab, ctx) {
            for c in p.commands {
                if let Some(k) = c.keys
                    && seen.insert(c.label)
                {
                    body.push_str(&format!("- **{k}**: {}\n", c.label));
                }
            }
        }
    }
    body.push_str(
        "- **Ctrl+N**, **Ctrl+O**, **Ctrl+S**: New, Open, Save\n\
         - **Ctrl+C**, **Ctrl+X**, **Ctrl+V**: Copy, Cut, Paste\n\
         - **Space**: the last ribbon command again\n\
         - **F1**: the help of the button under the pointer\n\
         - **F6**: the keyboard goes to the ribbon: **Tab** and **Shift+Tab** go from button to button, **Enter** or **Space** runs one, **Esc** goes back\n\
         - **Ctrl+Plus**, **Ctrl+Minus**, **Ctrl+0**: larger, smaller and normal text and buttons\n\
         \n## Canvas\n\n\
         - **Mouse wheel**: zoom at the pointer\n\
         - **Middle or right drag**: move the view\n\
         - **Home** or a double-click on the wheel: all of the design\n\
         - **Arrow keys**: move the selection a little (**Shift**: more)\n\
         - **Tab**, **Shift+Tab**: the next and the previous object\n\
         - **Delete**: delete the selection\n\
         - **Alt+V**: hide the selection on the screen; with nothing selected, show all\n\
         \n## Drawing\n\n\
         - **Left click**: a corner point. **Right click**: a curve point\n\
         - **Enter**: finish the object. **Backspace**: remove the last point. **Esc**: cancel or stop the tool\n\
         - **Ctrl** while you drag a rectangle or an ellipse: equal sides. **Alt**: from the center\n\
         \n## Stitch cursor\n\n\
         - **,** and **.**: one stitch back or forward (**Shift**: 10, **Ctrl**: 100, **Ctrl+Shift**: 1000)\n\
         - **Page Up**, **Page Down**: the previous or the next object\n\
         - **Ctrl+Page Up**, **Ctrl+Page Down**: the previous or the next color\n\
         - **Shift+Page Up**, **Shift+Page Down**: the previous or the next machine function (View > Stitch Cursor > Functions)\n\
         - **Alt+Page Up**, **Alt+Page Down**: the previous or the next segment\n",
    );
    Topic {
        id: KEYBOARD_ID.into(),
        title: "Keyboard Shortcuts".into(),
        group: Group::Reference,
        keywords: vec!["keyboard".into(), "shortcut".into(), "key".into(), "hotkey".into()],
        body,
    }
}

/// All topics: the files, the ribbon tabs and the keyboard.
pub fn all_topics(ctx: &egui::Context) -> Vec<Topic> {
    let mut v: Vec<Topic> = FILES.iter().filter_map(|f| parse_topic(f)).collect();
    v.extend(RibbonTab::ALL.iter().map(|t| ribbon_topic(*t, ctx)));
    v.push(keyboard_topic(ctx));
    v
}

// ---- Markdown ----

/// A block of a topic.
#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Heading(u8, String),
    Para(String),
    /// A list: numbered or not, and its items.
    List(bool, Vec<String>),
    /// A table: the rows of cells; the first row is the head.
    Table(Vec<Vec<String>>),
}

/// The cells of a table row (`| a | b |`); `None` for the line under the
/// head (`|---|---|`).
fn table_row(t: &str) -> Option<Vec<String>> {
    let inner = t.strip_prefix('|')?;
    let inner = inner.strip_suffix('|').unwrap_or(inner);
    let cells: Vec<String> = inner.split('|').map(|c| c.trim().to_string()).collect();
    let rule = cells
        .iter()
        .all(|c| !c.is_empty() && c.chars().all(|ch| matches!(ch, '-' | ':')));
    (!rule).then_some(cells)
}

/// The blocks of a Markdown text (see the module notes).
pub fn blocks(md: &str) -> Vec<Block> {
    let mut out: Vec<Block> = Vec::new();
    let mut para: Option<String> = None;
    let mut list: Option<(bool, Vec<String>)> = None;
    let mut table: Vec<Vec<String>> = Vec::new();
    let flush = |out: &mut Vec<Block>, para: &mut Option<String>, list: &mut Option<(bool, Vec<String>)>| {
        if let Some(p) = para.take() {
            out.push(Block::Para(p));
        }
        if let Some((n, items)) = list.take() {
            out.push(Block::List(n, items));
        }
    };
    for line in md.lines() {
        let t = line.trim();
        if t.starts_with('|') {
            flush(&mut out, &mut para, &mut list);
            table.extend(table_row(t));
            continue;
        }
        if !table.is_empty() {
            out.push(Block::Table(std::mem::take(&mut table)));
        }
        if t.is_empty() {
            flush(&mut out, &mut para, &mut list);
            continue;
        }
        let hashes = t.chars().take_while(|c| *c == '#').count();
        if (1..=3).contains(&hashes) && t[hashes..].starts_with(' ') {
            flush(&mut out, &mut para, &mut list);
            out.push(Block::Heading(hashes as u8, t[hashes..].trim().to_string()));
            continue;
        }
        let numbered = t
            .split_once(". ")
            .filter(|(n, _)| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()));
        let item = t
            .strip_prefix("- ")
            .map(|r| (false, r))
            .or(numbered.map(|(_, r)| (true, r)));
        match item {
            Some((n, text)) => {
                if let Some(p) = para.take() {
                    out.push(Block::Para(p));
                }
                match &mut list {
                    Some((m, items)) if *m == n => items.push(text.to_string()),
                    _ => {
                        if let Some((m, items)) = list.take() {
                            out.push(Block::List(m, items));
                        }
                        list = Some((n, vec![text.to_string()]));
                    }
                }
            }
            None => {
                // A line that goes on: the last list item, or the paragraph.
                if let Some((_, items)) = &mut list
                    && line.starts_with(' ')
                    && let Some(last) = items.last_mut()
                {
                    last.push(' ');
                    last.push_str(t);
                } else {
                    if let Some((m, items)) = list.take() {
                        out.push(Block::List(m, items));
                    }
                    match &mut para {
                        Some(p) => {
                            p.push(' ');
                            p.push_str(t);
                        }
                        None => para = Some(t.to_string()),
                    }
                }
            }
        }
    }
    flush(&mut out, &mut para, &mut list);
    if !table.is_empty() {
        out.push(Block::Table(table));
    }
    out
}

/// A part of a line of text.
#[derive(Debug, Clone, PartialEq)]
pub enum Span {
    Text(String),
    Bold(String),
    Code(String),
    /// The text and the topic id.
    Link(String, String),
}

/// The spans of a line of Markdown text.
pub fn spans(text: &str) -> Vec<Span> {
    let mut out = Vec::new();
    let mut rest = text;
    let mut plain = String::new();
    while !rest.is_empty() {
        if let Some(r) = rest.strip_prefix("**")
            && let Some(end) = r.find("**")
        {
            out.push(Span::Text(std::mem::take(&mut plain)));
            out.push(Span::Bold(r[..end].to_string()));
            rest = &r[end + 2..];
        } else if let Some(r) = rest.strip_prefix('`')
            && let Some(end) = r.find('`')
        {
            out.push(Span::Text(std::mem::take(&mut plain)));
            out.push(Span::Code(r[..end].to_string()));
            rest = &r[end + 1..];
        } else if let Some(r) = rest.strip_prefix('[')
            && let Some(mid) = r.find("](")
            && let Some(end) = r[mid..].find(')')
        {
            out.push(Span::Text(std::mem::take(&mut plain)));
            out.push(Span::Link(r[..mid].to_string(), r[mid + 2..mid + end].to_string()));
            rest = &r[mid + end + 1..];
        } else {
            let c = rest.chars().next().expect("not empty");
            plain.push(c);
            rest = &rest[c.len_utf8()..];
        }
    }
    out.push(Span::Text(plain));
    out.retain(|s| !matches!(s, Span::Text(t) if t.is_empty()));
    out
}

/// The text of a line of Markdown with no marks.
pub fn plain(text: &str) -> String {
    spans(text)
        .into_iter()
        .map(|s| match s {
            Span::Text(t) | Span::Bold(t) | Span::Code(t) | Span::Link(t, _) => t,
        })
        .collect()
}

// ---- Search ----

/// Words that do not help to find a topic (question words and the like).
const STOP: &[&str] = &[
    "a", "an", "the", "how", "do", "does", "did", "i", "can", "could", "what", "why", "where", "when", "which", "who",
    "is", "are", "was", "be", "to", "of", "in", "on", "at", "by", "my", "me", "it", "its", "this", "that", "these",
    "for", "with", "and", "or", "you", "your", "want", "there", "get", "should", "would", "will", "from", "as", "so",
    "if", "into", "than", "then", "some", "any", "use", "way", "make", "please",
];

/// Words of the same meaning: a word of a group finds the other words.
const SAME: &[&[&str]] = &[
    &[
        "large", "big", "bigger", "larger", "size", "scale", "resize", "height", "small", "smaller",
    ],
    &["color", "colour", "thread"],
    &["delete", "remove", "erase"],
    &["jump", "travel", "trim"],
    &["text", "letter", "lettering", "font", "name", "word"],
    &["picture", "image", "photo", "png", "jpeg", "jpg", "bitmap", "logo"],
    &["hoop", "frame"],
    &["export", "save", "write", "output"],
    &["machine", "format"],
    &["pucker", "wrinkle", "pull"],
    &["rotate", "turn"],
    &["mirror", "flip"],
    &["move", "drag", "shift"],
    &["undo", "back"],
    &["print", "worksheet", "sheet"],
    &["stone", "rhinestone", "bling"],
    &["fill", "tatami", "area"],
    &["satin", "column"],
    &["outline", "border"],
    &["dense", "density", "spacing"],
    &["shortcut", "key", "keyboard"],
    &["break", "broken", "stiff"],
    &["applique", "appliqué", "patch"],
    &["monogram", "initial", "initials"],
    &["quilt", "quilting", "stipple", "echo"],
    &["lace", "fsl", "ornament"],
    &["usb", "stick", "drive"],
    &["stabilizer", "backing", "topping"],
    &["towel", "pile", "terry", "fleece", "laydown"],
    &["cap", "hat"],
    &["shirt", "garment", "product"],
    &["outline", "contour", "trace"],
    &["weld", "merge", "combine", "unite"],
    &["birdnest", "loop", "tension"],
    &["gap", "registration", "shift"],
];

/// A word reduced to its stem (a simple English stemmer).
pub fn stem(word: &str) -> String {
    let w = word.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase();
    let w = w.strip_suffix("'s").unwrap_or(&w).to_string();
    let n = w.chars().count();
    if !w.is_ascii() || n <= 3 {
        return w;
    }
    let undouble = |s: &str| -> String {
        let b = s.as_bytes();
        if b.len() >= 2 && b[b.len() - 1] == b[b.len() - 2] && !matches!(b[b.len() - 1], b'l' | b's' | b'z') {
            s[..s.len() - 1].to_string()
        } else {
            s.to_string()
        }
    };
    let mut base = if n > 5 && w.ends_with("ing") {
        undouble(&w[..w.len() - 3])
    } else if n > 4 && w.ends_with("ies") {
        format!("{}y", &w[..w.len() - 3])
    } else if n > 4 && w.ends_with("ed") {
        undouble(&w[..w.len() - 2])
    } else if n > 4 && (w.ends_with("ches") || w.ends_with("shes") || w.ends_with("sses") || w.ends_with("xes")) {
        w[..w.len() - 2].to_string()
    } else if w.ends_with('s') && !w.ends_with("ss") && !w.ends_with("us") && !w.ends_with("is") {
        w[..w.len() - 1].to_string()
    } else {
        w
    };
    // The last e goes, so "save", "saved" and "saving" have the same stem.
    if base.len() > 3 && base.ends_with('e') {
        base.pop();
    }
    base
}

/// The stems of the words of a text, with the stop words left out.
pub fn tokens(text: &str) -> Vec<String> {
    text.split(|c: char| !(c.is_alphanumeric() || c == '\''))
        .filter(|w| !w.is_empty())
        .filter(|w| !STOP.contains(&w.to_lowercase().as_str()))
        .map(stem)
        .filter(|w| !w.is_empty())
        .collect()
}

/// A section of a topic: the text under one heading.
#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    pub topic: usize,
    pub heading: String,
    /// The paragraphs and list items, as plain text.
    pub passages: Vec<String>,
}

/// A search result.
#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    pub topic: usize,
    pub heading: String,
    /// The passage with most words of the question.
    pub snippet: String,
    pub score: f32,
}

/// The search index of the topics.
#[derive(Debug, Clone, Default)]
pub struct Index {
    pub sections: Vec<Section>,
    /// The weighted term counts of each section.
    terms: Vec<HashMap<String, f32>>,
    lens: Vec<f32>,
    df: HashMap<String, usize>,
    avg: f32,
}

impl Index {
    pub fn new(topics: &[Topic]) -> Index {
        let mut sections = Vec::new();
        for (t, topic) in topics.iter().enumerate() {
            let mut cur = Section {
                topic: t,
                heading: topic.title.clone(),
                passages: Vec::new(),
            };
            for b in blocks(&topic.body) {
                match b {
                    Block::Heading(1, _) => {}
                    Block::Heading(_, h) => {
                        if !cur.passages.is_empty() {
                            sections.push(cur);
                        }
                        cur = Section {
                            topic: t,
                            heading: plain(&h),
                            passages: Vec::new(),
                        };
                    }
                    Block::Para(p) => cur.passages.push(plain(&p)),
                    Block::List(_, items) => cur.passages.extend(items.iter().map(|i| plain(i))),
                    // Each row is a passage: "head: cell; head: cell".
                    Block::Table(rows) => {
                        let head = rows.first().cloned().unwrap_or_default();
                        for row in rows.iter().skip(1) {
                            let cells: Vec<String> = row
                                .iter()
                                .enumerate()
                                .map(|(i, c)| match head.get(i) {
                                    Some(h) if i > 0 => format!("{}: {}", plain(h), plain(c)),
                                    _ => plain(c),
                                })
                                .collect();
                            cur.passages.push(cells.join("; "));
                        }
                    }
                }
            }
            if !cur.passages.is_empty() {
                sections.push(cur);
            }
        }
        let mut terms = Vec::with_capacity(sections.len());
        let mut lens = Vec::with_capacity(sections.len());
        let mut df: HashMap<String, usize> = HashMap::new();
        for s in &sections {
            let topic = &topics[s.topic];
            let mut m: HashMap<String, f32> = HashMap::new();
            let mut add = |text: &str, w: f32| {
                for tok in tokens(text) {
                    *m.entry(tok).or_default() += w;
                }
            };
            add(&topic.title, 2.0);
            add(&s.heading, 3.0);
            add(&topic.keywords.join(" "), 1.0);
            for p in &s.passages {
                add(p, 1.0);
            }
            for k in m.keys() {
                *df.entry(k.clone()).or_default() += 1;
            }
            lens.push(m.values().sum());
            terms.push(m);
        }
        let avg = if lens.is_empty() {
            1.0
        } else {
            lens.iter().sum::<f32>() / lens.len() as f32
        };
        Index {
            sections,
            terms,
            lens,
            df,
            avg,
        }
    }

    /// The query terms with their weights: the words of the question (1)
    /// and the words of the same meaning (0.5).
    fn query_terms(q: &str) -> Vec<(String, f32)> {
        let mut out: Vec<(String, f32)> = Vec::new();
        for t in tokens(q) {
            if !out.iter().any(|(w, _)| *w == t) {
                out.push((t.clone(), 1.0));
            }
            for group in SAME {
                if group.iter().any(|g| stem(g) == t) {
                    for g in group.iter().map(|g| stem(g)) {
                        if !out.iter().any(|(w, _)| *w == g) {
                            out.push((g, 0.5));
                        }
                    }
                }
            }
        }
        out
    }

    /// The sections that answer `q`, the best first (at most `max`).
    pub fn search(&self, q: &str, max: usize) -> Vec<Hit> {
        const K1: f32 = 1.2;
        const B: f32 = 0.75;
        let terms = Index::query_terms(q);
        if terms.is_empty() {
            return Vec::new();
        }
        let n = self.sections.len() as f32;
        let mut scored: Vec<(usize, f32)> = (0..self.sections.len())
            .filter_map(|i| {
                let mut score = 0.0;
                for (t, w) in &terms {
                    let Some(&tf) = self.terms[i].get(t) else { continue };
                    let df = *self.df.get(t).unwrap_or(&0) as f32;
                    let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();
                    let norm = tf * (K1 + 1.0) / (tf + K1 * (1.0 - B + B * self.lens[i] / self.avg));
                    score += w * idf * norm;
                }
                (score > 0.0).then_some((i, score))
            })
            .collect();
        scored.sort_by(|a, b| b.1.total_cmp(&a.1));
        scored
            .into_iter()
            .take(max)
            .map(|(i, score)| {
                let s = &self.sections[i];
                let snippet = s
                    .passages
                    .iter()
                    .map(|p| {
                        let words: HashSet<String> = tokens(p).into_iter().collect();
                        let m: f32 = terms.iter().filter(|(t, _)| words.contains(t)).map(|(_, w)| w).sum();
                        (p, m)
                    })
                    .fold(None::<(&String, f32)>, |best, (p, m)| match best {
                        Some((_, bm)) if bm >= m => best,
                        _ => Some((p, m)),
                    })
                    .map(|(p, _)| p.clone())
                    .unwrap_or_default();
                Hit {
                    topic: s.topic,
                    heading: s.heading.clone(),
                    snippet,
                    score,
                }
            })
            .collect()
    }
}

// ---- The Help window ----

/// The state of the Help window.
#[derive(Default)]
pub struct HelpState {
    pub open: bool,
    /// The topics and the index, made when the window first shows.
    topics: Vec<Topic>,
    index: Index,
    /// The topic that shows.
    pub current: String,
    /// A text to show and mark in the topic (a heading or a command name).
    pub find: Option<String>,
    /// The mark scrolled into view already.
    scrolled: bool,
    pub query: String,
    /// The results of the last search; `None`: the topic shows.
    pub results: Option<Vec<Hit>>,
    /// The topics before, for Back.
    back: Vec<String>,
}

impl HelpState {
    /// Makes the topics and the index, once.
    pub fn ensure(&mut self, ctx: &egui::Context) {
        if self.topics.is_empty() {
            self.topics = all_topics(ctx);
            self.index = Index::new(&self.topics);
        }
    }

    pub fn search(&mut self, q: &str) {
        self.query = q.to_string();
        self.results = Some(self.index.search(q, 8));
    }

    fn go(&mut self, id: &str, find: Option<String>) {
        if self.current != id && !self.current.is_empty() {
            self.back.push(self.current.clone());
        }
        self.current = id.to_string();
        self.find = find;
        self.scrolled = false;
        self.results = None;
    }
}

impl AppState {
    /// Opens the Help window at topic `id`, and marks the text `find` in it.
    pub fn open_help(&mut self, id: &str, find: Option<&str>) {
        self.help.open = true;
        self.help.go(id, find.map(str::to_string));
    }

    /// Opens the Help window with the answers to a question.
    pub fn ask_help(&mut self, question: &str) {
        self.help.open = true;
        self.help.query = question.to_string();
        self.help_ask_pending = true;
    }

    /// F1: the help of the ribbon command under the pointer, of the window
    /// under the pointer (its topic, from [`window_topic`]), or of the
    /// active tool.
    pub fn help_f1(&mut self, window: Option<&str>) {
        if self.hover_cmd.is_none()
            && let Some(topic) = window
        {
            self.open_help(topic, None);
            return;
        }
        let cmd = self
            .hover_cmd
            .or_else(|| (self.tool != Tool::Select).then_some(Cmd::Tool(self.tool)));
        match cmd.and_then(|c| crate::ribbon::tab_of(c).map(|t| (c, t))) {
            Some((c, tab)) => {
                let label = c.meta().1;
                self.open_help(&ribbon_id(tab), Some(label));
            }
            None => self.open_help("getting-started", None),
        }
    }
}

/// Shows Markdown spans in a wrapped line. Returns a link that was clicked.
fn show_spans(ui: &mut egui::Ui, text: &str, strong: bool) -> Option<String> {
    let mut clicked = None;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for s in spans(text) {
            match s {
                Span::Text(t) => {
                    let r = RichText::new(t);
                    ui.label(if strong { r.strong() } else { r });
                }
                Span::Bold(t) => {
                    ui.label(RichText::new(t).strong());
                }
                Span::Code(t) => {
                    ui.label(RichText::new(t).code());
                }
                Span::Link(t, id) => {
                    if ui.link(t).clicked() {
                        clicked = Some(id);
                    }
                }
            }
        }
    });
    clicked
}

/// Shows a topic. Returns a link that was clicked.
fn show_topic(ui: &mut egui::Ui, topic: &Topic, find: Option<&str>, scrolled: &mut bool) -> Option<String> {
    let mut link = None;
    let find = find.map(str::to_lowercase);
    let mut marked = false;
    for b in blocks(&topic.body) {
        let text = match &b {
            Block::Heading(_, t) | Block::Para(t) => plain(t),
            Block::List(_, items) => items.iter().map(|i| plain(i)).collect::<Vec<_>>().join(" "),
            Block::Table(rows) => rows.iter().flatten().map(|c| plain(c)).collect::<Vec<_>>().join(" "),
        };
        let mark = !marked && find.as_ref().is_some_and(|f| text.to_lowercase().contains(f.as_str()));
        let mut draw = |ui: &mut egui::Ui| match &b {
            Block::Heading(level, t) => {
                ui.add_space(if *level == 1 { 2.0 } else { 8.0 });
                let size = match level {
                    1 => 20.0,
                    2 => 16.0,
                    _ => 14.0,
                };
                ui.label(RichText::new(plain(t)).size(size).strong());
            }
            Block::Para(t) => {
                if let Some(l) = show_spans(ui, t, false) {
                    link = Some(l);
                }
            }
            Block::List(numbered, items) => {
                for (k, item) in items.iter().enumerate() {
                    // In a long list, mark only the item with the text.
                    let item_mark = mark
                        && items.len() > 1
                        && find
                            .as_ref()
                            .is_some_and(|f| plain(item).to_lowercase().contains(f.as_str()));
                    let mut row = |ui: &mut egui::Ui| {
                        ui.horizontal_top(|ui| {
                            let bullet = if *numbered { format!("{}.", k + 1) } else { "•".into() };
                            ui.add_sized([18.0, 14.0], egui::Label::new(bullet));
                            if let Some(l) = show_spans(ui, item, false) {
                                link = Some(l);
                            }
                        })
                        .response
                    };
                    if item_mark {
                        let r = egui::Frame::new()
                            .fill(ui.visuals().selection.bg_fill.gamma_multiply(0.35))
                            .corner_radius(3.0)
                            .show(ui, |ui| row(ui))
                            .response;
                        if !*scrolled {
                            r.scroll_to_me(Some(egui::Align::Center));
                            *scrolled = true;
                        }
                    } else {
                        row(ui);
                    }
                }
            }
            Block::Table(rows) => {
                let cols = rows.iter().map(Vec::len).max().unwrap_or(1).max(1);
                let col_w = (ui.available_width() / cols as f32 - 12.0).max(60.0);
                egui::Grid::new(ui.next_auto_id())
                    .striped(true)
                    .spacing([12.0, 4.0])
                    .show(ui, |ui| {
                        for (r, row) in rows.iter().enumerate() {
                            for cell in row {
                                ui.scope(|ui| {
                                    ui.set_max_width(col_w);
                                    if let Some(l) = show_spans(ui, cell, r == 0) {
                                        link = Some(l);
                                    }
                                });
                            }
                            ui.end_row();
                        }
                    });
            }
        };
        let whole = mark && !matches!(&b, Block::List(_, items) if items.len() > 1);
        if whole {
            let r = egui::Frame::new()
                .fill(ui.visuals().selection.bg_fill.gamma_multiply(0.35))
                .corner_radius(3.0)
                .show(ui, |ui| draw(ui))
                .response;
            if !*scrolled {
                r.scroll_to_me(Some(egui::Align::TOP));
                *scrolled = true;
            }
        } else {
            draw(ui);
        }
        marked |= mark;
        ui.add_space(4.0);
    }
    link
}

/// The Help window.
pub fn window(ctx: &egui::Context, st: &mut AppState) {
    if !st.help.open {
        return;
    }
    st.help.ensure(ctx);
    if std::mem::take(&mut st.help_ask_pending) {
        let q = st.help.query.clone();
        st.help.search(&q);
    }
    if st.help.current.is_empty() {
        st.help.current = "getting-started".into();
    }
    let mut open = true;
    let mut go: Option<(String, Option<String>)> = None;
    let h = &mut st.help;
    egui::Window::new(tr("Help"))
        .id(egui::Id::new("Help"))
        .open(&mut open)
        .default_size([780.0, 560.0])
        .resizable(true)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(!h.back.is_empty(), egui::Button::new(tr("◀ Back")))
                    .clicked()
                    && let Some(prev) = h.back.pop()
                {
                    h.current = prev;
                    h.find = None;
                    h.results = None;
                }
                let r = ui.add(
                    egui::TextEdit::singleline(&mut h.query)
                        .hint_text(tr("Ask a question, for example: how do I make the letters larger"))
                        .desired_width(ui.available_width() - 70.0),
                );
                let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if ui.button(tr("Search")).clicked() || enter {
                    let q = h.query.clone();
                    h.search(&q);
                }
            });
            ui.separator();
            egui::Panel::left("help_contents")
                .resizable(true)
                .default_size(190.0)
                .show(ui, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for g in Group::ALL {
                            ui.label(RichText::new(g.label()).strong());
                            for t in h.topics.iter().filter(|t| t.group == g) {
                                let on = h.results.is_none() && t.id == h.current;
                                if ui.selectable_label(on, &t.title).clicked() {
                                    go = Some((t.id.clone(), None));
                                }
                            }
                            ui.add_space(6.0);
                        }
                    });
                });
            egui::ScrollArea::vertical()
                .auto_shrink(false)
                .show(ui, |ui| match &h.results {
                    Some(hits) if hits.is_empty() => {
                        ui.label(tr(
                            "No answer in the help. Try other words, or open a topic on the left.",
                        ));
                    }
                    Some(hits) => {
                        let best = &hits[0];
                        let topic = &h.topics[best.topic];
                        egui::Frame::group(ui.style()).show(ui, |ui| {
                            ui.label(RichText::new(tr("Answer")).strong());
                            show_spans(ui, &best.snippet, false);
                            if ui.link(format!("{} › {}", topic.title, best.heading)).clicked() {
                                go = Some((topic.id.clone(), Some(best.heading.clone())));
                            }
                        });
                        ui.add_space(8.0);
                        if hits.len() > 1 {
                            ui.label(RichText::new(tr("More")).strong());
                        }
                        for hit in &hits[1..] {
                            let topic = &h.topics[hit.topic];
                            if ui.link(format!("{} › {}", topic.title, hit.heading)).clicked() {
                                go = Some((topic.id.clone(), Some(hit.heading.clone())));
                            }
                            ui.weak(&hit.snippet);
                            ui.add_space(4.0);
                        }
                    }
                    None => {
                        if let Some(topic) = h.topics.iter().find(|t| t.id == h.current)
                            && let Some(id) = show_topic(ui, topic, h.find.as_deref(), &mut h.scrolled)
                        {
                            go = Some((id, None));
                        }
                    }
                });
        });
    if let Some((id, find)) = go {
        st.help.go(&id, find);
    }
    if !open {
        st.help.open = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::Harness;

    fn topics() -> Vec<Topic> {
        all_topics(&crate::test_util::ready_context())
    }

    /// On macOS the shortcuts show as symbols (⌘S); the topics of the
    /// ribbon and the keyboard are made there too.
    #[test]
    fn the_topics_are_made_on_macos_too() {
        let ctx = crate::test_util::ready_context();
        ctx.set_os(egui::os::OperatingSystem::Mac);
        let all = all_topics(&ctx);
        let keys = all.iter().find(|t| t.id == KEYBOARD_ID).expect("the keyboard topic");
        assert!(!keys.body.is_empty());
    }

    #[test]
    fn the_files_read_and_the_links_go_to_topics() {
        let all = topics();
        assert_eq!(
            all.iter()
                .filter(|t| t.group != Group::Ribbon && t.id != KEYBOARD_ID)
                .count(),
            FILES.len()
        );
        let ids: HashSet<&str> = all.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids.len(), all.len(), "the ids are unique");
        for t in &all {
            assert!(!t.body.is_empty(), "{}", t.id);
            for b in blocks(&t.body) {
                let texts = match b {
                    Block::Heading(_, s) | Block::Para(s) => vec![s],
                    Block::List(_, items) => items,
                    Block::Table(rows) => rows.into_iter().flatten().collect(),
                };
                for s in texts {
                    for sp in spans(&s) {
                        if let Span::Link(_, id) = sp {
                            assert!(ids.contains(id.as_str()), "{}: a link to {id}", t.id);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn each_ribbon_command_has_help() {
        let all = topics();
        for tab in RibbonTab::ALL {
            let t = all.iter().find(|t| t.id == ribbon_id(tab)).unwrap();
            for p in crate::ribbon::help_panels(tab, &crate::test_util::ready_context()) {
                assert!(t.body.contains(&format!("## {}", p.title)));
                for c in p.commands {
                    assert!(t.body.contains(&format!("**{}**", c.label)), "{}", c.label);
                }
            }
        }
    }

    #[test]
    fn markdown_blocks_and_spans() {
        let md = "# Title\n\nOne line\nand more.\n\n- A **bold** item\n  goes on\n- [Link](x)\n\n1. First\n2. Second\n";
        assert_eq!(
            blocks(md),
            vec![
                Block::Heading(1, "Title".into()),
                Block::Para("One line and more.".into()),
                Block::List(false, vec!["A **bold** item goes on".into(), "[Link](x)".into()]),
                Block::List(true, vec!["First".into(), "Second".into()]),
            ]
        );
        assert_eq!(
            spans("Push `F1` or **Esc**, see [this](topic)."),
            vec![
                Span::Text("Push ".into()),
                Span::Code("F1".into()),
                Span::Text(" or ".into()),
                Span::Bold("Esc".into()),
                Span::Text(", see ".into()),
                Span::Link("this".into(), "topic".into()),
                Span::Text(".".into()),
            ]
        );
    }

    #[test]
    fn stems() {
        for (w, s) in [
            ("stitches", "stitch"),
            ("stitched", "stitch"),
            ("stitching", "stitch"),
            ("running", "run"),
            ("letters", "letter"),
            ("puckers", "pucker"),
            ("colors", "color"),
            ("density", "density"),
            ("copies", "copy"),
            ("used", "used"),
            ("glass", "glass"),
            ("saved", "sav"),
            ("saving", "sav"),
            ("save", "sav"),
        ] {
            assert_eq!(stem(w), s, "{w}");
        }
        assert_eq!(tokens("How do I make the letters larger?"), vec!["letter", "larger"]);
    }

    #[test]
    fn the_search_answers_questions() {
        let all = topics();
        let index = Index::new(&all);
        let top = |q: &str| {
            let hits = index.search(q, 5);
            assert!(!hits.is_empty(), "{q}");
            (
                all[hits[0].topic].id.clone(),
                hits[0].heading.clone(),
                hits[0].snippet.clone(),
            )
        };
        let (id, heading, _) = top("how do I make the letters larger");
        assert_eq!(
            (id.as_str(), heading.as_str()),
            ("tutorial-lettering", "Change the size and the spacing")
        );
        assert_eq!(top("my fabric puckers").0, "troubleshooting");
        assert_eq!(
            top("the machine does not read my file").1,
            "The machine does not read the file"
        );
        let (id, _, snippet) = top("paste at the stitch cursor");
        assert!(["ribbon-edit", "editing", "checking"].contains(&id.as_str()), "{id}");
        assert!(snippet.to_lowercase().contains("paste"), "{snippet}");
        assert_eq!(top("rhinestone template for a cutter").0, "rhinestones");
        assert_eq!(top("keyboard shortcut to undo").0, KEYBOARD_ID);
        // Questions of the topics of ADR 0055.
        assert_eq!(top("how do I sew a design larger than the hoop").0, "hooping");
        assert_eq!(top("stitches sink into the towel").0, "troubleshooting");
        let id = top("what is pull compensation").0;
        assert!(
            ["glossary", "embroidery-basics", "objects-and-stitches"].contains(&id.as_str()),
            "{id}"
        );
        assert_eq!(top("which formats can the program read").0, "file-formats");
        assert_eq!(top("echo lines round an appliqué").0, "quilting");
        assert!(index.search("the of and", 5).is_empty(), "only stop words");
    }

    #[test]
    fn f1_opens_the_help_of_the_command_under_the_pointer() {
        let mut h = Harness::empty();
        h.st.hover_cmd = Some(Cmd::Paste);
        h.st.help_f1(None);
        assert!(h.st.help.open);
        assert_eq!(h.st.help.current, "ribbon-edit");
        assert_eq!(h.st.help.find.as_deref(), Some("Paste"));
        // No command: the active tool.
        h.st.hover_cmd = None;
        h.st.set_tool(Tool::Column);
        h.st.help_f1(None);
        assert_eq!(h.st.help.current, "ribbon-digitize");
        assert_eq!(h.st.help.find.as_deref(), Some("Column"));
        // Nothing: Getting started.
        h.st.set_tool(Tool::Select);
        h.st.help_f1(None);
        assert_eq!(h.st.help.current, "getting-started");
    }

    #[test]
    fn f1_over_a_window_opens_its_topic() {
        let ids: HashSet<String> = topics().into_iter().map(|t| t.id).collect();
        for (_, topic) in WINDOW_HELP {
            assert!(ids.contains(*topic), "{topic}");
        }
        // A window with its own id, under the pointer.
        let ctx = crate::test_util::ready_context();
        let frame = |pointer: Option<egui::Pos2>| {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000.0, 800.0))),
                events: pointer.map(egui::Event::PointerMoved).into_iter().collect(),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
                egui::Window::new(tr("Offsets"))
                    .id(egui::Id::new("Offsets"))
                    .fixed_pos([100.0, 100.0])
                    .show(ui.ctx(), |ui| ui.label("Offsets"));
            });
            out.textures_delta.clear();
        };
        frame(None);
        frame(Some(egui::pos2(120.0, 120.0)));
        assert_eq!(window_topic(&ctx), Some("artwork"));
        frame(Some(egui::pos2(900.0, 700.0)));
        assert_eq!(window_topic(&ctx), None);
        // The window topic comes before the tool, the ribbon button first.
        let mut h = Harness::empty();
        h.st.set_tool(Tool::Column);
        h.st.help_f1(Some("artwork"));
        assert_eq!(h.st.help.current, "artwork");
        h.st.hover_cmd = Some(Cmd::Paste);
        h.st.help_f1(Some("artwork"));
        assert_eq!(h.st.help.current, "ribbon-edit");
    }

    #[test]
    fn markdown_tables() {
        let md = "Text\n\n| Format | Read |\n|---|:-:|\n| DST | **yes** |\n| PES | no |\n\nMore.";
        assert_eq!(
            blocks(md),
            vec![
                Block::Para("Text".into()),
                Block::Table(vec![
                    vec!["Format".into(), "Read".into()],
                    vec!["DST".into(), "**yes**".into()],
                    vec!["PES".into(), "no".into()],
                ]),
                Block::Para("More.".into()),
            ]
        );
        // Each row is a passage of the search, with the head of each cell.
        let t = Topic {
            id: "t".into(),
            title: "T".into(),
            group: Group::Reference,
            keywords: vec![],
            body: md.into(),
        };
        let index = Index::new(&[t]);
        assert!(index.sections[0].passages.contains(&"DST; Read: yes".to_string()));
    }

    #[test]
    fn the_window_draws_topics_and_answers() {
        let mut h = Harness::empty();
        let ctx = egui::Context::default();
        let frame = |st: &mut AppState| {
            let mut out = ctx.run_ui(Default::default(), |ui| {
                let ctx = ui.ctx().clone();
                window(&ctx, st);
            });
            out.textures_delta.clear();
        };
        h.st.open_help("ribbon-edit", Some("Paste Special…"));
        frame(&mut h.st);
        frame(&mut h.st);
        assert!(h.st.help.scrolled, "the mark is in view");
        h.st.ask_help("how do I change the thread color");
        frame(&mut h.st);
        let hits = h.st.help.results.clone().unwrap();
        assert!(!hits.is_empty());
        frame(&mut h.st);
    }
}
