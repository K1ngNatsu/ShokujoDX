//! Lettering in the app (ADR 0015, 0016): the font library (the bundled
//! embroidery and TrueType fonts, the user's embroidery fonts and the fonts
//! of the computer), the Lettering tool and its window, Object Properties
//! for lettering objects (with the baseline and the selected letter, ADR
//! 0017), and Lettering on line.
//!
//! The fonts of the computer are found in a background thread, so the app
//! starts with the bundled fonts and adds the others when the scan is done.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};

use eframe::egui;
use emb_geom::{Point, Transform};
use emb_letter::{AnyFont, EmbFont, Font};
use emb_model::{
    Baseline, EnvelopeShape, FontKind, FontRef, Justify, LetterConnection, LetterEdit, LetterOrder, LetterStitch,
    Lettering, MergeKey, ObjectId, ObjectKind, Underlay,
};

use crate::app::AppState;
use crate::i18n::tr;
use crate::manipulate::ViewMap;
use crate::words;

/// Font files larger than this are not read (large Asian fonts).
const MAX_FONT_BYTES: u64 = 50 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub enum FontSource {
    /// An index into `emb_letter::bundled::FILES`.
    Bundled(usize),
    /// A font file and the face in it.
    File(PathBuf, u32),
    /// An index into `emb_letter::bundled::EMBROIDERY`.
    EmbBundled(usize),
    /// An embroidery font file of the user.
    EmbFile(PathBuf),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FontEntry {
    pub font: FontRef,
    pub source: FontSource,
}

/// The fonts that lettering can use.
#[derive(Debug)]
pub struct FontLibrary {
    pub entries: Vec<FontEntry>,
    loaded: HashMap<FontRef, AnyFont>,
    scan: Option<Receiver<Vec<FontEntry>>>,
    /// The fonts of the computer are in `entries`.
    pub scanned: bool,
}

impl Default for FontLibrary {
    fn default() -> Self {
        let mut entries: Vec<FontEntry> = emb_letter::bundled::EMBROIDERY
            .iter()
            .enumerate()
            .map(|(i, (family, _))| FontEntry {
                font: FontRef::embroidery(family, ""),
                source: FontSource::EmbBundled(i),
            })
            .collect();
        entries.extend(
            emb_letter::bundled::FILES
                .iter()
                .enumerate()
                .filter_map(|(i, (_, bytes))| {
                    let (family, style) = emb_letter::face_names(bytes, 0)?;
                    Some(FontEntry {
                        font: FontRef::new(&family, &style),
                        source: FontSource::Bundled(i),
                    })
                }),
        );
        let mut lib = FontLibrary {
            entries,
            loaded: HashMap::new(),
            scan: None,
            scanned: false,
        };
        if let Some(dir) = user_fonts_dir() {
            lib.add_user_fonts(&dir);
        }
        lib
    }
}

/// The folder of the user's embroidery fonts, next to the settings file.
pub fn user_fonts_dir() -> Option<PathBuf> {
    crate::workspace::settings_path()?.parent().map(|d| d.join("fonts"))
}

impl FontLibrary {
    /// Starts to find the fonts of the computer (once).
    pub fn start_scan(&mut self) {
        if self.scanned || self.scan.is_some() {
            return;
        }
        let (tx, rx) = channel();
        self.scan = Some(rx);
        std::thread::spawn(move || {
            tx.send(scan(&font_dirs())).ok();
        });
    }

    /// Takes the result of the scan when it is ready.
    pub fn poll(&mut self) {
        let Some(rx) = &self.scan else { return };
        if let Ok(found) = rx.try_recv() {
            for e in found {
                if !self.entries.iter().any(|x| x.font.same(&e.font)) {
                    self.entries.push(e);
                }
            }
            self.sort();
            self.scan = None;
            self.scanned = true;
        }
    }

    /// Embroidery fonts first, then by name.
    fn sort(&mut self) {
        self.entries.sort_by(|a, b| {
            let key = |e: &FontEntry| {
                (
                    e.font.kind != FontKind::Embroidery,
                    e.font.family.to_lowercase(),
                    e.font.style.clone(),
                )
            };
            key(a).cmp(&key(b))
        });
    }

    /// Adds the embroidery fonts in `dir` (and keeps them loaded).
    pub fn add_user_fonts(&mut self, dir: &Path) {
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        for path in rd.flatten().map(|e| e.path()) {
            if path.extension().and_then(|e| e.to_str()) == Some(emb_letter::embfont::FILE_EXTENSION) {
                self.add_user_font(&path).ok();
            }
        }
    }

    /// Adds one embroidery font file. A font with the same name is replaced.
    pub fn add_user_font(&mut self, path: &Path) -> Result<FontRef, String> {
        let f = EmbFont::load(path)?;
        let r = FontRef::embroidery(&f.family, &f.style);
        self.entries.retain(|e| !e.font.same(&r));
        self.loaded.retain(|k, _| !k.same(&r));
        self.entries.push(FontEntry {
            font: r.clone(),
            source: FontSource::EmbFile(path.to_path_buf()),
        });
        self.loaded.insert(r.clone(), AnyFont::Embroidery(Arc::new(f)));
        self.sort();
        Ok(r)
    }

    /// Removes a font (a deleted or renamed alphabet) from the lists.
    pub fn forget(&mut self, r: &FontRef) {
        self.entries.retain(|e| !e.font.same(r));
        self.loaded.retain(|k, _| !k.same(r));
    }

    /// The font, read when it is first used. `None` if the computer does
    /// not have it.
    pub fn get(&mut self, r: &FontRef) -> Option<AnyFont> {
        if let Some((_, f)) = self.loaded.iter().find(|(k, _)| k.same(r)) {
            return Some(f.clone());
        }
        let e = self.entries.iter().find(|e| e.font.same(r))?.clone();
        let font = match e.source {
            FontSource::Bundled(i) => AnyFont::TrueType(Arc::new(
                Font::from_bytes(emb_letter::bundled::FILES.get(i)?.1.to_vec(), 0).ok()?,
            )),
            FontSource::File(path, face) => {
                AnyFont::TrueType(Arc::new(Font::from_bytes(std::fs::read(path).ok()?, face).ok()?))
            }
            FontSource::EmbBundled(i) => AnyFont::Embroidery(Arc::new(
                EmbFont::from_json(&emb_letter::bundled::embroidery_text(i)?).ok()?,
            )),
            FontSource::EmbFile(path) => AnyFont::Embroidery(Arc::new(EmbFont::load(&path).ok()?)),
        };
        self.loaded.insert(e.font, font.clone());
        Some(font)
    }

    /// The default font (a bundled embroidery font).
    pub fn default_font(&mut self) -> Option<AnyFont> {
        self.get(&FontRef::embroidery(emb_letter::bundled::DEFAULT_EMBROIDERY, ""))
    }

    /// The families, embroidery fonts first, each with its kind.
    pub fn families(&self) -> Vec<(String, FontKind)> {
        let mut v: Vec<(String, FontKind)> = Vec::new();
        for e in &self.entries {
            if !v
                .iter()
                .any(|(f, k)| *k == e.font.kind && f.eq_ignore_ascii_case(&e.font.family))
            {
                v.push((e.font.family.clone(), e.font.kind));
            }
        }
        v
    }

    pub fn styles(&self, family: &str, kind: FontKind) -> Vec<String> {
        self.entries
            .iter()
            .filter(|e| e.font.kind == kind && e.font.family.eq_ignore_ascii_case(family))
            .map(|e| e.font.style.clone())
            .collect()
    }
}

/// The folders where the operating system keeps fonts.
fn font_dirs() -> Vec<PathBuf> {
    let env = |k: &str| std::env::var_os(k).map(PathBuf::from);
    let mut v = Vec::new();
    if cfg!(windows) {
        v.extend(env("WINDIR").map(|w| w.join("Fonts")));
        v.extend(env("LOCALAPPDATA").map(|l| l.join("Microsoft").join("Windows").join("Fonts")));
    } else if cfg!(target_os = "macos") {
        v.push(PathBuf::from("/System/Library/Fonts"));
        v.push(PathBuf::from("/Library/Fonts"));
        v.extend(env("HOME").map(|h| h.join("Library").join("Fonts")));
    } else {
        v.push(PathBuf::from("/usr/share/fonts"));
        v.push(PathBuf::from("/usr/local/share/fonts"));
        v.extend(env("HOME").map(|h| h.join(".local").join("share").join("fonts")));
        v.extend(env("HOME").map(|h| h.join(".fonts")));
    }
    v
}

/// The font faces in `dirs` and their subfolders.
pub fn scan(dirs: &[PathBuf]) -> Vec<FontEntry> {
    fn walk(dir: &Path, depth: usize, out: &mut Vec<FontEntry>) {
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        for entry in rd.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if depth < 5 {
                    walk(&path, depth + 1, out);
                }
                continue;
            }
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(str::to_ascii_lowercase)
                .unwrap_or_default();
            if !matches!(ext.as_str(), "ttf" | "otf" | "ttc" | "otc") {
                continue;
            }
            if entry.metadata().map_or(true, |m| m.len() > MAX_FONT_BYTES) {
                continue;
            }
            let Ok(bytes) = std::fs::read(&path) else { continue };
            for face in 0..emb_letter::face_count(&bytes).min(16) {
                if let Some((family, style)) = emb_letter::face_names(&bytes, face) {
                    out.push(FontEntry {
                        font: FontRef::new(&family, &style),
                        source: FontSource::File(path.clone(), face),
                    });
                }
            }
        }
    }
    let mut out = Vec::new();
    for d in dirs {
        walk(d, 0, &mut out);
    }
    out
}

/// The window that opens when the Lettering tool clicks on the canvas.
#[derive(Debug, Clone)]
pub struct LetteringDialog {
    /// The start of the baseline of the first line.
    pub at: Point,
    pub text: String,
    pub font: FontRef,
    pub height_mm: f64,
    focus: bool,
    /// The lettering object that the window changes (double-click); `None`:
    /// the window adds a new one.
    pub edit: Option<ObjectId>,
}

impl LetteringDialog {
    pub fn new(at: Point, text: &str, font: FontRef, height_mm: f64) -> LetteringDialog {
        LetteringDialog {
            at,
            text: text.to_string(),
            font,
            height_mm,
            focus: true,
            edit: None,
        }
    }

    /// The window for an existing lettering object.
    pub fn for_object(id: ObjectId, l: &Lettering) -> LetteringDialog {
        LetteringDialog {
            edit: Some(id),
            ..LetteringDialog::new(l.placement.apply(Point::ZERO), &l.text, l.font.clone(), l.height_mm)
        }
    }
}

/// An object that a command waits for: the user clicks it on the canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pick {
    /// Lettering on line: the line for this lettering object.
    LineFor(ObjectId),
    /// Lettering on line: the lettering object for this line.
    LetteringFor(ObjectId),
}

impl Pick {
    pub fn prompt(self) -> &'static str {
        match self {
            Pick::LineFor(_) => "Click the line that becomes the baseline. Esc: cancel.",
            Pick::LetteringFor(_) => "Click the lettering to put on this line. Esc: cancel.",
        }
    }
}

/// Canvas input of the Lettering tool: a click opens the Lettering window.
pub fn tool_input(response: &egui::Response, st: &mut AppState, map: &ViewMap) {
    if response.clicked()
        && let Some(pos) = response.interact_pointer_pos()
    {
        let at = st.snap(map, map.to_design(pos));
        st.fonts.start_scan();
        st.lettering_dialog = Some(LetteringDialog::new(at, "", st.letter_font.clone(), st.letter_height));
    }
}

/// Makes the letters of `l` with its font. Returns `false` (and leaves the
/// letters as they are) if the computer does not have the font.
pub fn relayout(st: &mut AppState, l: &mut Lettering) -> bool {
    match st.fonts.get(&l.font) {
        Some(font) => {
            emb_letter::relayout(l, font.source());
            true
        }
        None => false,
    }
}

impl AppState {
    /// Puts the selected lettering on the selected line: the line becomes
    /// the baseline of the lettering and is no longer an object.
    pub fn lettering_on_line(&mut self) {
        let d = self.doc.design();
        let objs: Vec<_> = self.selection.ids().iter().filter_map(|id| d.object(*id)).collect();
        let lettering = objs.iter().find_map(|o| match &o.kind {
            ObjectKind::Lettering(l) => Some((o.id, (**l).clone())),
            _ => None,
        });
        let line = objs.iter().find_map(|o| match &o.kind {
            ObjectKind::Running { path, .. } => Some((o.id, path.clone())),
            _ => None,
        });
        let pick = match (lettering, line, objs.len()) {
            (Some((id, _)), Some((line_id, _)), 2) => return self.put_on_line(id, line_id),
            (Some((id, _)), None, 1) => Pick::LineFor(id),
            (None, Some((line_id, _)), 1) => Pick::LetteringFor(line_id),
            _ => {
                self.message = tr("Select a lettering object (or a line), then use Lettering on line.").into();
                return;
            }
        };
        self.message = pick.prompt().into();
        self.pick = Some(pick);
    }

    /// A click on object `id` while a command waits for it. Returns `true`
    /// if the click was for the command.
    pub fn pick_clicked(&mut self, id: Option<ObjectId>) -> bool {
        let Some(pick) = self.pick else { return false };
        let kind = |st: &AppState, id: ObjectId| st.doc.design().object(id).map(|o| o.kind.clone());
        match (pick, id.and_then(|i| kind(self, i).map(|k| (i, k)))) {
            (Pick::LineFor(l), Some((line, ObjectKind::Running { .. }))) => {
                self.pick = None;
                self.put_on_line(l, line);
            }
            (Pick::LetteringFor(line), Some((l, ObjectKind::Lettering(_)))) => {
                self.pick = None;
                self.put_on_line(l, line);
            }
            (Pick::LineFor(_), _) => {
                self.message = tr("That is not a line. Click a running-stitch line, or press Esc.").into()
            }
            (Pick::LetteringFor(_), _) => {
                self.message = tr("That is not lettering. Click a lettering object, or press Esc.").into()
            }
        }
        true
    }

    /// Makes line `line_id` the baseline of lettering `id`; the line is no
    /// longer an object.
    pub fn put_on_line(&mut self, id: ObjectId, line_id: ObjectId) {
        let d = self.doc.design();
        let (Some(ObjectKind::Lettering(l)), Some(ObjectKind::Running { path, .. })) =
            (d.object(id).map(|o| &o.kind), d.object(line_id).map(|o| &o.kind))
        else {
            return;
        };
        let (mut l, path) = ((**l).clone(), path.clone());
        let Some(inv) = l.placement.inverse() else { return };
        l.baseline = Baseline::Path {
            path: path.transformed(&inv),
        };
        if !relayout(self, &mut l) {
            self.message = crate::i18n::fill("{} is not on this computer: choose another font.", &[&(l.font.label())]);
            return;
        }
        self.doc.edit("Lettering on Line", |d| {
            if let Some(o) = d.object_mut(id) {
                o.kind = ObjectKind::Lettering(Box::new(l));
            }
            d.remove_object(line_id);
        });
        self.selection.set(id);
        self.message = tr("The line is now the baseline. Reshape tool: drag its nodes to change it.").into();
    }

    /// What right-click > Edit opens for the selection: the Monogram window
    /// for a monogram, the Lettering window for one lettering object.
    pub fn edit_target(&self) -> Option<EditTarget> {
        let d = self.doc.design();
        let ids = self.selection.ids();
        let objs: Vec<&emb_model::DesignObject> = ids.iter().filter_map(|id| d.object(*id)).collect();
        if objs
            .iter()
            .filter_map(|o| o.group)
            .any(|g| d.monograms.iter().any(|r| r.group == g))
        {
            return Some(EditTarget::Monogram);
        }
        let mut letterings = objs.iter().filter(|o| matches!(o.kind, ObjectKind::Lettering(_)));
        match (letterings.next(), letterings.next()) {
            (Some(o), None) => Some(EditTarget::Lettering(o.id)),
            _ => None,
        }
    }

    /// Right-click > Edit: opens the window that edits the selection.
    /// Returns `false` if the selection has nothing to edit there.
    pub fn edit_selection(&mut self) -> bool {
        match self.edit_target() {
            Some(EditTarget::Monogram) => self.open_monogram(),
            Some(EditTarget::Lettering(id)) => self.edit_lettering(id),
            None => return false,
        }
        true
    }

    /// Opens the Lettering window for lettering object `id` (double-click).
    pub fn edit_lettering(&mut self, id: ObjectId) {
        if let Some(ObjectKind::Lettering(l)) = self.doc.design().object(id).map(|o| &o.kind) {
            self.fonts.start_scan();
            self.lettering_dialog = Some(LetteringDialog::for_object(id, l));
        }
    }

    /// Changes the text, the font and the height of the object of the
    /// window. The letters keep their changes by hand where the text stays.
    pub fn apply_lettering_dialog(&mut self, d: LetteringDialog) {
        let Some(id) = d.edit else {
            return self.add_lettering(d);
        };
        let Some(ObjectKind::Lettering(l)) = self.doc.design().object(id).map(|o| o.kind.clone()) else {
            return;
        };
        let mut n = (*l).clone();
        n.retarget_edits(&d.text);
        n.text = d.text.clone();
        n.height_mm = d.height_mm;
        if self.fonts.get(&d.font).is_some() {
            n.font = d.font.clone();
        }
        if !relayout(self, &mut n) {
            self.message = crate::i18n::fill("{} is not on this computer: choose another font.", &[&(n.font.label())]);
            return;
        }
        if n.letters.is_empty() {
            self.message = crate::i18n::fill("{} has none of these characters.", &[&(n.font.label())]);
            return;
        }
        self.letter_font = n.font.clone();
        self.letter_height = n.height_mm;
        self.reshape.letter = None;
        self.doc.edit("Change lettering", |dd| {
            if let Some(o) = dd.object_mut(id) {
                o.kind = ObjectKind::Lettering(Box::new(n));
            }
        });
    }

    /// Adds a lettering object from the Lettering window.
    pub fn add_lettering(&mut self, d: LetteringDialog) {
        let font = match self.fonts.get(&d.font) {
            Some(f) => f,
            None => match self.fonts.default_font() {
                Some(f) => f,
                None => return,
            },
        };
        let mut l = Lettering::new(&d.text, font.source().font_ref(), d.height_mm, d.at);
        emb_letter::relayout(&mut l, font.source());
        self.letter_font = l.font.clone();
        self.letter_height = d.height_mm;
        if l.letters.is_empty() {
            self.message = crate::i18n::fill("{} has none of these characters.", &[&(l.font.label())]);
            return;
        }
        self.add_drawn(ObjectKind::Lettering(Box::new(l)));
    }
}

/// What right-click > Edit opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditTarget {
    /// The Monogram window, to change the selected monogram.
    Monogram,
    /// The Lettering window, to change the text of this lettering object.
    Lettering(ObjectId),
}

/// The Lettering window, and the font scan.
pub fn windows(ctx: &egui::Context, st: &mut AppState) {
    st.fonts.poll();
    if st.fonts.scan.is_some() {
        ctx.request_repaint_after(std::time::Duration::from_millis(200));
    }
    let Some(mut d) = st.lettering_dialog.take() else {
        return;
    };
    let mut open = true;
    let mut done = false;
    let mut cancel = false;
    crate::dock::docked(
        egui::Window::new(if d.edit.is_some() {
            tr("Edit Lettering")
        } else {
            tr("Lettering")
        })
        .id(egui::Id::new("Lettering")),
        ctx,
        egui::Id::new("Lettering"),
        st.canvas_rect,
    )
    .open(&mut open)
    .collapsible(false)
    .resizable(false)
    .default_width(320.0)
    .show(ctx, |ui| {
        ui.label(tr("Text"));
        let r = ui.add(
            egui::TextEdit::multiline(&mut d.text)
                .desired_rows(3)
                .desired_width(300.0)
                .hint_text(tr("Type the text. Enter: new line.")),
        );
        if d.focus {
            r.request_focus();
            d.focus = false;
        }
        let ctrl_enter = r.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter) && i.modifiers.command);
        font_picker(ui, st, &mut d.font, "dialog");
        ui.horizontal(|ui| {
            ui.label(tr("Height"))
                .on_hover_text(tr("The height of the capital letters."));
            ui.add(
                egui::DragValue::new(&mut d.height_mm)
                    .range(2.0..=300.0)
                    .clamp_existing_to_range(false)
                    .speed(0.2)
                    .suffix(" mm"),
            );
        });
        ui.separator();
        ui.horizontal(|ui| {
            let ok = ui
                .add_enabled(!d.text.trim().is_empty(), egui::Button::new(tr("OK")))
                .on_hover_text(tr("Ctrl+Enter"));
            if ok.clicked() || (ctrl_enter && !d.text.trim().is_empty()) {
                done = true;
            }
            if ui.button(tr("Cancel")).clicked() {
                cancel = true;
            }
        });
    });
    if done {
        // Ctrl+Enter may have added a new line to the text.
        let text = d.text.trim_end_matches(['\n', '\r']).to_string();
        st.apply_lettering_dialog(LetteringDialog { text, ..d });
    } else if open && !cancel {
        st.lettering_dialog = Some(d);
    }
}

/// "embroidery" or "TrueType", for lists.
pub fn kind_name(k: FontKind) -> &'static str {
    match k {
        FontKind::Embroidery => tr("embroidery"),
        FontKind::TrueType => tr("TrueType"),
    }
}

/// Family and style lists. Returns `true` if the font changed.
pub fn font_picker(ui: &mut egui::Ui, st: &mut AppState, font: &mut FontRef, salt: &str) -> bool {
    st.fonts.start_scan();
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(tr("Font"));
        let families = st.fonts.families();
        egui::ComboBox::from_id_salt(("font_family", salt))
            .selected_text(&font.family)
            .width(180.0)
            .height(320.0)
            .show_ui(ui, |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut st.font_filter)
                        .hint_text(tr("Search"))
                        .desired_width(160.0),
                );
                if !st.fonts.scanned {
                    ui.weak(tr("Finding the fonts of this computer…"));
                }
                let filter = st.font_filter.to_lowercase();
                let mut last_kind = None;
                for (f, kind) in families.iter().filter(|(f, _)| f.to_lowercase().contains(&filter)) {
                    if last_kind != Some(*kind) {
                        ui.weak(match kind {
                            FontKind::Embroidery => "Embroidery fonts",
                            FontKind::TrueType => "TrueType fonts",
                        });
                        last_kind = Some(*kind);
                    }
                    let on = font.kind == *kind && font.family.eq_ignore_ascii_case(f);
                    if ui.selectable_label(on, f).clicked() {
                        let styles = st.fonts.styles(f, *kind);
                        let style = styles
                            .iter()
                            .find(|s| s.eq_ignore_ascii_case(&font.style))
                            .or_else(|| styles.iter().find(|s| s.eq_ignore_ascii_case("regular")))
                            .or(styles.first())
                            .cloned()
                            .unwrap_or_default();
                        *font = FontRef {
                            family: f.clone(),
                            style,
                            kind: *kind,
                        };
                        changed = true;
                    }
                }
            });
        let styles = st.fonts.styles(&font.family, font.kind);
        if styles.len() > 1 {
            egui::ComboBox::from_id_salt(("font_style", salt))
                .selected_text(&font.style)
                .show_ui(ui, |ui| {
                    for s in styles {
                        if ui.selectable_label(font.style.eq_ignore_ascii_case(&s), &s).clicked() {
                            font.style = s;
                            changed = true;
                        }
                    }
                });
        }
    });
    changed
}

/// The stitch type list of TrueType lettering.
fn stitch_combo(ui: &mut egui::Ui, stitch: &mut LetterStitch) {
    ui.horizontal(|ui| {
        ui.label(tr("Stitch type"));
        egui::ComboBox::from_id_salt("letter_stitch")
            .selected_text(stitch_name(*stitch))
            .show_ui(ui, |ui| {
                for s in [LetterStitch::Auto, LetterStitch::Satin, LetterStitch::Fill] {
                    ui.selectable_value(stitch, s, stitch_name(s));
                }
            })
            .response
            .on_hover_text(tr(
                "Automatic: satin, but fill for letters with strokes too wide for satin.",
            ));
    });
}

fn justify_name(j: Justify) -> &'static str {
    match j {
        Justify::Left => tr("Left"),
        Justify::Center => tr("Center"),
        Justify::Right => tr("Right"),
    }
}

/// The kinds of baseline in the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BaselineKind {
    Straight,
    ArcOver,
    ArcUnder,
    CircleCw,
    CircleCcw,
    Line,
}

impl BaselineKind {
    const ALL: [BaselineKind; 6] = [
        BaselineKind::Straight,
        BaselineKind::ArcOver,
        BaselineKind::ArcUnder,
        BaselineKind::CircleCw,
        BaselineKind::CircleCcw,
        BaselineKind::Line,
    ];

    fn of(b: &Baseline) -> BaselineKind {
        match b {
            Baseline::Straight => BaselineKind::Straight,
            Baseline::Arc { clockwise: true, .. } => BaselineKind::ArcOver,
            Baseline::Arc { clockwise: false, .. } => BaselineKind::ArcUnder,
            Baseline::Circle { clockwise: true, .. } => BaselineKind::CircleCw,
            Baseline::Circle { clockwise: false, .. } => BaselineKind::CircleCcw,
            Baseline::Path { .. } => BaselineKind::Line,
        }
    }

    fn name(self) -> &'static str {
        match self {
            BaselineKind::Straight => tr("Straight"),
            BaselineKind::ArcOver => tr("Arc over"),
            BaselineKind::ArcUnder => tr("Arc under"),
            BaselineKind::CircleCw => tr("Circle, clockwise"),
            BaselineKind::CircleCcw => tr("Circle, counter-clockwise"),
            BaselineKind::Line => tr("Line"),
        }
    }

    fn help(self) -> &'static str {
        match self {
            BaselineKind::Straight => "The letters stand on a straight line.",
            BaselineKind::ArcOver => "The letters stand on top of an arc. Drag the center of the arc to change it.",
            BaselineKind::ArcUnder => "The letters hang under an arc. Drag the center of the arc to change it.",
            BaselineKind::CircleCw => "The letters go round a circle, outside it, from the top.",
            BaselineKind::CircleCcw => "The letters go round a circle, inside it, from the bottom.",
            BaselineKind::Line => {
                "Draw a line (Run tool), select the lettering, use Lettering > Lettering > \
                 Lettering on line, then click the line."
            }
        }
    }
}

/// Changes the baseline of `l` to `kind` and keeps the text where it is.
pub(crate) fn set_baseline(l: &mut Lettering, kind: BaselineKind) {
    let old = l.baseline.clone();
    let width: f64 = l.letters.iter().filter(|x| x.line == 0).map(|x| 2.0 * x.center.x).sum();
    let radius = old
        .radius()
        .unwrap_or_else(|| ((width / 2.1).max(2.0 * l.height_mm) * 2.0).round() / 2.0);
    let new = match kind {
        BaselineKind::Straight => Baseline::Straight,
        BaselineKind::ArcOver | BaselineKind::ArcUnder => Baseline::Arc {
            radius_mm: radius,
            clockwise: kind == BaselineKind::ArcOver,
        },
        BaselineKind::CircleCw | BaselineKind::CircleCcw => Baseline::Circle {
            radius_mm: radius,
            clockwise: kind == BaselineKind::CircleCw,
        },
        BaselineKind::Line => return,
    };
    // From a straight line to a curve: the middle of the text goes to the
    // top (or the bottom) of the curve.
    if old.is_straight() && !new.is_straight() {
        let mid = match l.justify {
            Justify::Left => width / 2.0,
            Justify::Center => 0.0,
            Justify::Right => -width / 2.0,
        };
        l.placement = Transform::translate(mid, 0.0).then(l.placement);
        l.justify = Justify::Center;
    }
    // The start of the old line and the new line are at the same place.
    let (a, b) = (old.frame(0.0, 0.0, 0.5).0, new.frame(0.0, 0.0, 0.5).0);
    l.placement = Transform::translate(a.x - b.x, a.y - b.y).then(l.placement);
    l.baseline = new;
}

/// The baseline list and the radius. Returns `true` on a change.
fn baseline_ui(ui: &mut egui::Ui, n: &mut Lettering, drag: &mut bool) -> bool {
    let mut changed = false;
    let now = BaselineKind::of(&n.baseline);
    ui.horizontal(|ui| {
        ui.label(tr("Baseline"))
            .on_hover_text(tr("The line that the letters stand on."));
        egui::ComboBox::from_id_salt("letter_baseline")
            .selected_text(tr(now.name()))
            .show_ui(ui, |ui| {
                for k in BaselineKind::ALL {
                    let enabled = k != BaselineKind::Line || now == BaselineKind::Line;
                    let r = ui
                        .add_enabled(enabled, egui::Button::selectable(now == k, tr(k.name())))
                        .on_hover_text(k.help())
                        .on_disabled_hover_text(k.help());
                    if r.clicked() && k != now {
                        set_baseline(n, k);
                        changed = true;
                    }
                }
            });
    });
    if let Baseline::Arc { radius_mm, .. } | Baseline::Circle { radius_mm, .. } = &mut n.baseline {
        ui.horizontal(|ui| {
            ui.label(tr("Radius"))
                .on_hover_text(tr("The radius of the baseline circle."));
            let r = ui.add(
                egui::DragValue::new(radius_mm)
                    .range(emb_model::baseline::MIN_RADIUS_MM..=2000.0)
                    .clamp_existing_to_range(false)
                    .speed(0.2)
                    .suffix(" mm"),
            );
            *drag |= r.dragged();
            changed |= r.changed();
        });
    }
    changed
}

/// The values of the selected letter. Returns `Some(true)` if the change
/// needs a new layout, `Some(false)` if the letter moves only.
fn letter_ui(ui: &mut egui::Ui, n: &mut Lettering, letter: usize, drag: &mut bool) -> Option<bool> {
    let true_type = n.font.kind == FontKind::TrueType;
    let l = n.letters.get(letter)?.clone();
    let old = n.edit(l.index).cloned().unwrap_or(LetterEdit::new(l.index));
    let mut e = old.clone();
    ui.separator();
    ui.strong(format!(
        "Letter \u{201c}{}\u{201d} ({} of {})",
        l.text,
        letter + 1,
        n.letters.len()
    ));
    let mut value = |ui: &mut egui::Ui, label: &str, help: &str, v: &mut f64, speed: f64, suffix: &str| {
        ui.horizontal(|ui| {
            ui.label(label).on_hover_text(help);
            let r = ui.add(egui::DragValue::new(v).speed(speed).suffix(suffix).max_decimals(2));
            *drag |= r.dragged();
        });
    };
    value(
        ui,
        tr("Kerning"),
        tr("Extra space before the letter. The letters after it on the line move too."),
        &mut e.kern_mm,
        0.05,
        " mm",
    );
    value(
        ui,
        tr("Move along"),
        tr("Move the letter along the baseline."),
        &mut e.offset.x,
        0.05,
        " mm",
    );
    value(
        ui,
        tr("Move down"),
        tr("Move the letter down from the baseline."),
        &mut e.offset.y,
        0.05,
        " mm",
    );
    let mut size = e.scale * 100.0;
    value(ui, tr("Size"), tr("The size of this letter."), &mut size, 0.5, " %");
    e.scale = (size / 100.0).clamp(0.1, 10.0);
    value(
        ui,
        tr("Turn"),
        tr("Turn the letter about its middle."),
        &mut e.rotate_deg,
        0.5,
        "\u{b0}",
    );
    if true_type {
        ui.horizontal(|ui| {
            ui.label(tr("Stitch type")).on_hover_text(tr(
                "The stitch type of this letter. As the object: the stitch type of the lettering.",
            ));
            egui::ComboBox::from_id_salt("one_letter_stitch")
                .selected_text(e.stitch.map_or(tr("As the object"), stitch_name))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut e.stitch, None, tr("As the object"));
                    for s in [LetterStitch::Auto, LetterStitch::Satin, LetterStitch::Fill] {
                        ui.selectable_value(&mut e.stitch, Some(s), stitch_name(s));
                    }
                });
        });
    }
    ui.horizontal(|ui| {
        ui.label(tr("Underlay")).on_hover_text(tr(words::UNDERLAY_HELP));
        egui::ComboBox::from_id_salt("one_letter_underlay")
            .selected_text(e.underlay.as_ref().map_or(tr("As the object"), words::underlay))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut e.underlay, None, tr("As the object"));
                for u in [Underlay::Light, Underlay::Auto, Underlay::Heavy, Underlay::None] {
                    let name = words::underlay(&u);
                    ui.selectable_value(&mut e.underlay, Some(u), name);
                }
            });
    });
    if e.shape.is_some() {
        ui.weak(tr("The shape of this letter was changed by hand."));
    }
    let mut reset = false;
    ui.horizontal(|ui| {
        reset = ui
            .add_enabled(!old.is_empty(), egui::Button::new(tr("Reset letter")))
            .on_hover_text(tr("Remove all changes by hand from this letter."))
            .clicked();
        if ui
            .add_enabled(!n.edits.is_empty(), egui::Button::new(tr("Reset all letters")))
            .clicked()
        {
            n.edits.clear();
            reset = true;
        }
    });
    if reset {
        n.reset_letter(l.index);
        return Some(true);
    }
    if e == old {
        return None;
    }
    if e.kern_mm != old.kern_mm || e.stitch != old.stitch {
        *n.edit_mut(l.index) = e;
        n.prune_edits();
        return Some(true);
    }
    n.change_letter(letter, |x| *x = e);
    Some(false)
}

/// A small picture of an envelope shape.
fn envelope_icon(ui: &mut egui::Ui, shape: EnvelopeShape, on: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(40.0, 30.0), egui::Sense::click());
    let v = ui.visuals();
    let stroke = if on {
        v.selection.stroke
    } else if resp.hovered() {
        v.widgets.hovered.fg_stroke
    } else {
        v.widgets.inactive.fg_stroke
    };
    if on {
        ui.painter()
            .rect_filled(rect, 3.0, v.selection.bg_fill.gamma_multiply(0.4));
    }
    let e = emb_model::Envelope { shape, amount: 0.6 };
    let inner = rect.shrink2(egui::vec2(5.0, 9.0));
    let h = inner.height();
    let (mut top, mut bottom) = (Vec::new(), Vec::new());
    for i in 0..=16 {
        let u = i as f64 / 16.0;
        let (up, down) = e.edges(u);
        let x = inner.left() + inner.width() * u as f32;
        top.push(egui::pos2(x, inner.top() - up as f32 * h));
        bottom.push(egui::pos2(x, inner.bottom() + down as f32 * h));
    }
    let mut outline = top.clone();
    outline.extend(bottom.iter().rev());
    ui.painter()
        .add(egui::Shape::closed_line(outline, egui::Stroke::new(1.2, stroke.color)));
    resp.on_hover_text(tr(shape.name()))
}

/// Lettering art: the envelope gallery and its amount. Returns `true` if
/// the letters need a new layout.
fn envelope_ui(ui: &mut egui::Ui, n: &mut Lettering, drag: &mut bool) -> bool {
    ui.separator();
    ui.strong(tr("Lettering art"))
        .on_hover_text(tr("Bend the text into a shape (an envelope)."));
    if !n.baseline.is_straight() {
        ui.weak(tr("Lettering art needs a straight baseline."));
        return false;
    }
    let mut changed = false;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
        for shape in EnvelopeShape::ALL {
            let on = n.envelope.shape == shape;
            if envelope_icon(ui, shape, on).clicked() && !on {
                n.envelope.shape = shape;
                if n.envelope.amount == 0.0 {
                    n.envelope.amount = emb_model::Envelope::default().amount;
                }
                changed = true;
            }
        }
    });
    if n.envelope.shape != EnvelopeShape::None {
        ui.horizontal(|ui| {
            ui.label(tr("Amount"))
                .on_hover_text(tr("How far the edges of the text move, as a part of the text height."));
            let mut pct = n.envelope.amount * 100.0;
            let r = ui.add(
                egui::Slider::new(&mut pct, 0.0..=emb_model::envelope::MAX_AMOUNT * 100.0)
                    .clamping(egui::SliderClamping::Edits)
                    .suffix(" %")
                    .fixed_decimals(0),
            );
            if r.changed() {
                n.envelope.amount = pct / 100.0;
                changed = true;
            }
            *drag |= r.dragged();
        });
    }
    changed
}

fn order_name(o: LetterOrder) -> &'static str {
    match o {
        LetterOrder::Text => tr("In text order"),
        LetterOrder::CenterOut => tr("Center out"),
        LetterOrder::Closest => tr("Closest Join"),
    }
}

fn order_help(o: LetterOrder) -> &'static str {
    match o {
        LetterOrder::Text => tr("The letters sew in the order of the text, each part as it comes."),
        LetterOrder::CenterOut => tr(
            "From the middle letter to the right end, then from the middle to the left end. \
             For caps. The parts of each letter sew with as few trims as possible.",
        ),
        LetterOrder::Closest => tr(
            "The nearest letter next. The parts of each letter sew with as few trims as possible: \
             a travel stitch where the next stitches cover it.",
        ),
    }
}

/// The sew order and the connections of the letters (no new layout).
fn order_ui(ui: &mut egui::Ui, n: &mut Lettering, drag: &mut bool) {
    ui.separator();
    ui.strong(tr("Sew Order"));
    ui.horizontal(|ui| {
        ui.label(tr("Order"));
        egui::ComboBox::from_id_salt("letter_order")
            .selected_text(order_name(n.order))
            .show_ui(ui, |ui| {
                for o in [LetterOrder::Text, LetterOrder::CenterOut, LetterOrder::Closest] {
                    ui.selectable_value(&mut n.order, o, order_name(o))
                        .on_hover_text(order_help(o));
                }
            })
            .response
            .on_hover_text(order_help(n.order));
    });
    ui.horizontal(|ui| {
        ui.label(tr("Trim gaps longer than"))
            .on_hover_text(tr("A shorter gap between two parts gets a travel stitch."));
        let r = ui.add(
            egui::DragValue::new(&mut n.trim_over_mm)
                .range(0.0..=20.0)
                .clamp_existing_to_range(false)
                .speed(0.05)
                .suffix(" mm"),
        );
        *drag |= r.dragged();
    });
    let mut jump = n.connection == LetterConnection::Jump;
    if ui
        .checkbox(&mut jump, tr("Jumps, no trims"))
        .on_hover_text(tr(
            "For machines with no trimmer: the long gaps are jumps. Cut them by hand.",
        ))
        .changed()
    {
        n.connection = if jump {
            LetterConnection::Jump
        } else {
            LetterConnection::Trim
        };
    }
}

fn stitch_name(s: LetterStitch) -> &'static str {
    match s {
        LetterStitch::Auto => tr("Automatic"),
        LetterStitch::Satin => tr("Satin"),
        LetterStitch::Fill => tr("Fill"),
    }
}

/// Object Properties of a lettering object.
pub fn properties(ui: &mut egui::Ui, st: &mut AppState, id: ObjectId, l: &Lettering) {
    let mut n = l.clone();
    let mut drag = false;
    let has_font = st.fonts.get(&l.font).is_some();
    ui.label(tr("Text"));
    let text = ui.add(
        egui::TextEdit::multiline(&mut n.text)
            .desired_rows(2)
            .desired_width(f32::INFINITY),
    );
    let typing = text.has_focus();
    if !has_font {
        ui.colored_label(
            ui.visuals().warn_fg_color,
            format!(
                "{} is not on this computer. The letters stay as they are until you choose another font.",
                l.font.label()
            ),
        );
    }
    font_picker(ui, st, &mut n.font, "properties");
    let mut slider = |ui: &mut egui::Ui,
                      label: &str,
                      help: &str,
                      v: &mut f64,
                      range: std::ops::RangeInclusive<f64>,
                      suffix: &str| {
        ui.horizontal(|ui| {
            ui.label(label).on_hover_text(help);
            let r = ui.add(
                egui::Slider::new(v, range)
                    .clamping(egui::SliderClamping::Edits)
                    .suffix(suffix)
                    .fixed_decimals(2),
            );
            drag |= r.dragged();
        });
    };
    slider(
        ui,
        tr("Height"),
        tr("The height of the capital letters."),
        &mut n.height_mm,
        2.0..=150.0,
        tr(" mm"),
    );
    if let Some((lo, hi)) = l.range_mm {
        let text = crate::i18n::fill(
            "Size range of the font: {}–{} mm",
            &[&format!("{lo:.0}"), &format!("{hi:.0}")],
        );
        if l.size_warning().is_some() {
            ui.colored_label(ui.visuals().warn_fg_color, text);
        } else {
            ui.weak(text);
        }
    }
    slider(
        ui,
        tr("Letter spacing"),
        tr("Extra space after each letter."),
        &mut n.letter_spacing_mm,
        -5.0..=20.0,
        tr(" mm"),
    );
    if n.text.contains('\n') {
        slider(
            ui,
            tr("Line spacing"),
            tr("The distance between lines, as a part of the normal line distance of the font."),
            &mut n.line_spacing,
            0.5..=3.0,
            "×",
        );
    }
    ui.horizontal(|ui| {
        ui.label(tr("Align")).on_hover_text(tr(
            "Where the text starts on the baseline. On a line: at its start, middle or end.",
        ));
        for j in [Justify::Left, Justify::Center, Justify::Right] {
            ui.selectable_value(&mut n.justify, j, justify_name(j));
        }
    });
    let mut layout = baseline_ui(ui, &mut n, &mut drag);
    layout |= envelope_ui(ui, &mut n, &mut drag);
    order_ui(ui, &mut n, &mut drag);
    ui.checkbox(&mut n.kerning, tr("Kerning")).on_hover_text(tr(
        "Use the letter pairs of the font (for example \"AV\") to set the space between letters.",
    ));
    if n.font.kind == FontKind::Embroidery {
        ui.weak(tr("The letters of an embroidery font keep their own stitch types."));
    } else {
        stitch_combo(ui, &mut n.stitch);
    }
    match crate::letter_edit::selected_letter(st) {
        Some(i) => {
            if let Some(needs) = letter_ui(ui, &mut n, i, &mut drag) {
                layout |= needs;
            }
        }
        None => {
            ui.weak(tr("Reshape tool: select a letter to move, turn, scale or reshape it."));
        }
    }
    let (satins, fills) = n.parts().fold((0, 0), |(s, f), p| match p {
        ObjectKind::Satin { .. } => (s + 1, f),
        ObjectKind::Fill { .. } => (s, f + 1),
        _ => (s, f),
    });
    if satins > 0 {
        ui.separator();
        ui.strong(tr("Satin"));
        let mut row = |ui: &mut egui::Ui,
                       label: &str,
                       v: &mut f64,
                       range: std::ops::RangeInclusive<f64>,
                       word: crate::applique::Word| {
            ui.horizontal(|ui| {
                ui.label(label).on_hover_text(word.1);
                let r = ui.add(
                    egui::Slider::new(v, range)
                        .clamping(egui::SliderClamping::Edits)
                        .suffix(" mm")
                        .fixed_decimals(2),
                );
                ui.weak(format!("({})", (word.0)(*v)));
                drag |= r.dragged();
            });
        };
        row(
            ui,
            tr("Density"),
            &mut n.satin.spacing_mm,
            0.2..=1.0,
            (words::density, tr(words::DENSITY_HELP)),
        );
        row(
            ui,
            tr("Pull compensation"),
            &mut n.satin.pull_compensation_mm,
            0.0..=1.0,
            (words::pull, tr(words::PULL_HELP)),
        );
        crate::panels::underlay_combo(ui, "letter_satin_underlay", &mut n.satin.underlay);
    }
    if fills > 0 {
        ui.separator();
        ui.strong(tr("Fill"));
        ui.horizontal(|ui| {
            ui.label(tr("Density")).on_hover_text(tr(words::DENSITY_HELP));
            let r = ui.add(
                egui::Slider::new(&mut n.fill.spacing_mm, 0.2..=1.0)
                    .clamping(egui::SliderClamping::Edits)
                    .suffix(" mm")
                    .fixed_decimals(2),
            );
            ui.weak(format!("({})", words::density(n.fill.spacing_mm)));
            drag |= r.dragged();
        });
        ui.horizontal(|ui| {
            ui.label(tr("Angle"));
            let r = ui.add(
                egui::Slider::new(&mut n.fill.angle_deg, 0.0..=179.0)
                    .clamping(egui::SliderClamping::Edits)
                    .suffix("°")
                    .fixed_decimals(0),
            );
            drag |= r.dragged();
        });
        crate::panels::underlay_combo(ui, "letter_fill_underlay", &mut n.fill.underlay);
    }

    // Only the merge of this panel ends here: a drag on the canvas (move,
    // turn, scale) keeps its one undo step.
    let key = MergeKey::new(format!("{}:lettering", id.0));
    if n == *l {
        if !typing && !drag {
            st.doc.end_merge_of(&key);
        }
        return;
    }
    if n.text != l.text {
        // The changed characters lose their edits; the selected letter may
        // be another one now.
        let mut old = l.clone();
        old.retarget_edits(&n.text);
        n.edits = old.edits;
        st.reshape.letter = None;
    }
    // Text, font, size, baseline and kerning need the font; stitch
    // settings and moves of one letter do not.
    let shape_changed = layout
        || n.text != l.text
        || n.font != l.font
        || n.height_mm != l.height_mm
        || n.letter_spacing_mm != l.letter_spacing_mm
        || n.line_spacing != l.line_spacing
        || n.justify != l.justify
        || n.kerning != l.kerning
        || n.stitch != l.stitch;
    if shape_changed && !relayout(st, &mut n) {
        st.message = crate::i18n::fill("{} is not on this computer: choose another font.", &[&(n.font.label())]);
        return;
    }
    if !shape_changed {
        n.apply_params();
    }
    if n.letters.is_empty() {
        st.message = crate::i18n::fill("{} has none of these characters.", &[&(n.font.label())]);
    }
    st.letter_font = n.font.clone();
    st.letter_height = n.height_mm;
    st.doc.edit_merged("Change lettering", key.clone(), |d| {
        if let Some(o) = d.object_mut(id) {
            o.kind = ObjectKind::Lettering(Box::new(n));
        }
    });
    if !typing && !drag {
        st.doc.end_merge_of(&key);
    }
}

/// "Lettering · Lato Bold · 25 mm · Anna", for the Sequence panel.
pub fn summary(l: &Lettering) -> String {
    let mut text: String = l.text.replace('\n', " ").chars().take(24).collect();
    if l.text.chars().count() > 24 {
        text.push('…');
    }
    format!("{} · {:.0} mm · {text}", l.font.label(), l.height_mm)
}

#[cfg(test)]
mod tests {
    use emb_model::{HoopCatalog, MachineCatalog};

    use super::*;

    #[test]
    fn bundled_fonts_are_listed_and_load() {
        let mut lib = FontLibrary::default();
        assert!(
            lib.families()
                .iter()
                .any(|(f, k)| f == "Lato" && *k == FontKind::TrueType)
        );
        assert_eq!(lib.styles("Lato", FontKind::TrueType).len(), 2);
        assert_eq!(lib.families()[0].1, FontKind::Embroidery, "embroidery fonts first");
        assert!(lib.default_font().is_some());
        assert!(
            lib.get(&FontRef::new("lato", "bold")).is_some(),
            "names match in any case"
        );
        assert!(lib.get(&FontRef::new("No such font", "Regular")).is_none());
    }

    #[test]
    fn scan_finds_font_files() {
        let dir = std::env::temp_dir().join(format!("emb-fonts-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("sub").join("p.TTF"), emb_letter::bundled::FILES[2].1).unwrap();
        std::fs::write(dir.join("not-a-font.ttf"), b"nothing").unwrap();
        let found = scan(std::slice::from_ref(&dir));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].font.family, "Pacifico");
        std::fs::remove_dir_all(dir).ok();
    }

    fn only_lettering(st: &AppState) -> (ObjectId, Lettering) {
        let o = st
            .doc
            .design()
            .objects()
            .iter()
            .find(|o| matches!(o.kind, ObjectKind::Lettering(_)))
            .unwrap();
        let ObjectKind::Lettering(l) = &o.kind else { panic!() };
        (o.id, (**l).clone())
    }

    fn letters_bbox(l: &Lettering) -> emb_geom::BBox {
        l.parts()
            .filter_map(|p| p.control_bbox())
            .reduce(emb_geom::BBox::union)
            .unwrap()
    }

    #[test]
    fn a_circle_baseline_keeps_the_text_in_place() {
        let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
        st.add_lettering(LetteringDialog::new(
            Point::new(-30.0, 10.0),
            "TEAM",
            FontRef::embroidery("Block Sans", ""),
            15.0,
        ));
        let (_, l) = only_lettering(&st);
        let before = letters_bbox(&l);
        let mut n = l.clone();
        set_baseline(&mut n, BaselineKind::CircleCw);
        assert!(relayout(&mut st, &mut n));
        assert_eq!(n.justify, Justify::Center);
        let r = n.baseline.radius().unwrap();
        assert!(r >= 30.0, "{r}");
        let after = letters_bbox(&n);
        // The middle of the text stays; the top of the circle is on the old
        // baseline.
        assert!(
            (after.center().x - before.center().x).abs() < 1.0,
            "{before:?} {after:?}"
        );
        let center = n.placement.apply(Point::ZERO);
        assert!((center.y - (10.0 + r)).abs() < 1e-6, "{center:?}");
        // And back: the text is on a straight line at the same place.
        set_baseline(&mut n, BaselineKind::Straight);
        assert!(relayout(&mut st, &mut n));
        let back = letters_bbox(&n);
        assert!(back.center().distance(before.center()) < 0.5, "{before:?} {back:?}");
        // Arc over and arc under share the anchor point.
        set_baseline(&mut n, BaselineKind::ArcOver);
        let a = n.placement.apply(n.baseline.frame(0.0, 0.0, 0.5).0);
        set_baseline(&mut n, BaselineKind::ArcUnder);
        let b = n.placement.apply(n.baseline.frame(0.0, 0.0, 0.5).0);
        assert!(a.distance(b) < 1e-9);
    }

    #[test]
    fn lettering_on_a_line() {
        let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
        st.add_lettering(LetteringDialog::new(
            Point::new(-30.0, 0.0),
            "Wave",
            FontRef::embroidery("Block Sans", ""),
            10.0,
        ));
        let line = st.doc.edit("Line", |d| {
            let t = d.add_thread("Red", emb_model::Rgb(200, 0, 0));
            d.add_object(
                t,
                ObjectKind::Running {
                    path: emb_geom::Path::new().move_to(Point::new(-40.0, 30.0)).cubic_to(
                        Point::new(-10.0, 0.0),
                        Point::new(10.0, 60.0),
                        Point::new(40.0, 30.0),
                    ),
                    params: Default::default(),
                },
            )
        });
        let (id, _) = only_lettering(&st);
        // With the lettering selected, the command waits for a click on a line.
        st.selection.set(id);
        assert!(st.can(crate::ribbon::Cmd::LetteringOnLine, &crate::ribbon::SelInfo::of(&st)));
        st.lettering_on_line();
        assert_eq!(st.pick, Some(Pick::LineFor(id)));
        assert!(st.pick_clicked(Some(id)), "a click on the lettering is not a line");
        assert_eq!(st.pick, Some(Pick::LineFor(id)));
        assert!(st.pick_clicked(Some(line)));
        assert!(st.pick.is_none());
        assert_eq!(st.doc.design().objects().len(), 1, "the line is now the baseline");
        let (_, l) = only_lettering(&st);
        assert!(matches!(l.baseline, Baseline::Path { .. }));
        // The text starts at the start of the line: the base point of the
        // first letter is half its width along the line.
        let c = l.letters[0].center;
        let base = l.letter_to_design(0).unwrap().apply(Point::new(c.x, 0.0));
        let start = Point::new(-40.0, 30.0);
        assert!((base.distance(start) - c.x).abs() < 1.0, "{base:?} {}", c.x);
        st.doc.undo();
        assert_eq!(st.doc.design().objects().len(), 2);
        // Both selected: at once.
        st.selection.set_all([id, line]);
        st.lettering_on_line();
        assert_eq!(st.doc.design().objects().len(), 1);
    }

    #[test]
    fn esc_closes_the_lettering_window_from_its_text_box() {
        use eframe::egui::{Event, Key, Modifiers, Pos2, RawInput, Rect};
        let mut h = crate::test_util::Harness::empty();
        h.st.lettering_dialog = Some(LetteringDialog::new(
            Point::ZERO,
            "Ann",
            FontRef::embroidery("Block Sans", ""),
            15.0,
        ));
        let frame = |h: &mut crate::test_util::Harness, events: Vec<Event>| {
            let input = RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, crate::test_util::SCREEN)),
                events,
                ..Default::default()
            };
            let st = &mut h.st;
            let mut out = h.ctx.run_ui(input, |ui| {
                st.handle_shortcuts(&ui.ctx().clone());
                windows(ui.ctx(), st);
            });
            out.textures_delta.clear();
        };
        // The text box takes the focus when the window opens.
        frame(&mut h, vec![]);
        frame(&mut h, vec![]);
        assert!(h.ctx.memory(|m| m.focused().is_some()));
        let esc = Event::Key {
            key: Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        };
        frame(&mut h, vec![esc]);
        assert!(h.st.lettering_dialog.is_none());
        assert!(h.st.doc.design().objects().is_empty(), "nothing added");
        frame(&mut h, vec![]);
        assert!(h.ctx.memory(|m| m.focused().is_none()));
    }

    #[test]
    fn double_click_edits_the_text() {
        let mut h = crate::test_util::Harness::empty();
        h.st.add_lettering(LetteringDialog::new(
            Point::new(-20.0, 5.0),
            "Anna",
            FontRef::embroidery("Block Sans", ""),
            15.0,
        ));
        let id = h.id(0);
        h.st.zoom_to = Some(emb_geom::BBox::new(Point::new(-30.0, -20.0), Point::new(30.0, 15.0)));
        h.frame(vec![]);
        h.st.selection.clear();
        // Double-click the first stroke of the "A".
        let (_, l) = only_lettering(&h.st);
        let at = l.letters[0].parts[0].control_bbox().unwrap().center();
        let pos = h.screen(at);
        for _ in 0..2 {
            h.click(eframe::egui::PointerButton::Primary, pos);
        }
        let d = h.st.lettering_dialog.clone().expect("the window opens");
        assert_eq!((d.edit, d.text.as_str()), (Some(id), "Anna"));
        h.st.lettering_dialog = None;
        h.st.apply_lettering_dialog(LetteringDialog {
            text: "Anne".into(),
            ..d
        });
        let (_, l) = only_lettering(&h.st);
        assert_eq!(l.text, "Anne");
        assert_eq!(h.st.doc.design().objects().len(), 1, "the same object changed");
        h.st.undo();
        assert_eq!(only_lettering(&h.st).1.text, "Anna");
    }

    #[test]
    fn a_turn_on_the_canvas_is_one_undo_step_with_the_panel_open() {
        let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
        st.add_lettering(LetteringDialog::new(
            Point::new(0.0, 0.0),
            "Anna",
            FontRef::embroidery("Block Sans", ""),
            15.0,
        ));
        let (id, _) = only_lettering(&st);
        st.selection.set(id);
        let steps = st.doc.undo_len();
        let ctx = egui::Context::default();
        let key = MergeKey::new("canvas-rotate");
        for deg in 1..=30 {
            // A frame of a rotate drag, then the panel draws the turned object.
            st.doc.edit_merged("Rotate", key.clone(), |d| {
                d.transform_objects(&[id], &emb_model::edit::rotate_about(Point::ZERO, f64::from(deg)))
            });
            let (_, l) = only_lettering(&st);
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| properties(ui, &mut st, id, &l));
            out.textures_delta.clear();
        }
        assert_eq!(st.doc.undo_len(), steps + 1, "one step for the whole turn");
    }

    #[test]
    fn space_repeats_the_last_command() {
        let mut h = crate::test_util::Harness::empty();
        let ctx = h.ctx.clone();
        h.st.run(crate::ribbon::Cmd::Tool(crate::draw::Tool::Rectangle), &ctx);
        h.st.set_tool(crate::draw::Tool::Select);
        h.key(eframe::egui::Key::Space);
        assert_eq!(h.st.tool, crate::draw::Tool::Rectangle);
        // Undo is not a command to repeat.
        h.st.run(crate::ribbon::Cmd::Undo, &ctx);
        assert_eq!(
            h.st.last_cmd,
            Some(crate::ribbon::Cmd::Tool(crate::draw::Tool::Rectangle))
        );
    }

    #[test]
    fn add_lettering_and_change_it_with_undo() {
        let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
        st.add_lettering(LetteringDialog {
            at: Point::new(-30.0, 10.0),
            text: "Anna".into(),
            font: FontRef::new("Lato", "Bold"),
            height_mm: 20.0,
            focus: false,
            edit: None,
        });
        let d = st.doc.design().clone();
        assert_eq!(d.objects().len(), 1);
        let o = &d.objects()[0];
        let ObjectKind::Lettering(l) = &o.kind else { panic!() };
        assert_eq!(l.letters.len(), 4);
        let b = o.control_bbox().unwrap();
        // Round letters go a little below the baseline.
        assert!(
            (b.max.y - 10.0).abs() < 1.0,
            "the baseline is where the click was: {b:?}"
        );
        assert!(b.min.x >= -30.5);
        let plan = st.plan_view().plan.clone();
        assert!(plan.stitches.stitch_count() > 300);
        assert!(plan.stitches.trim_count() >= 3, "a trim between the letters");
        // The warnings of the object (the joins of the satin letters of the
        // bundled font build up, a place of the density map; ADR 0044).
        let issues: Vec<String> = st
            .design_info()
            .issues
            .iter()
            .filter(|i| i.object.is_some())
            .map(|i| i.text.clone())
            .collect();
        assert!(issues.is_empty(), "trims between letters are not a warning: {issues:?}");

        // Change the text in the properties path: relayout keeps the placement.
        let id = o.id;
        let mut n = (**l).clone();
        n.text = "Ann".into();
        assert!(relayout(&mut st, &mut n));
        st.doc.edit("Change lettering", |d| {
            d.object_mut(id).unwrap().kind = ObjectKind::Lettering(Box::new(n))
        });
        let ObjectKind::Lettering(l) = &st.doc.design().objects()[0].kind else {
            panic!()
        };
        assert_eq!(l.letters.len(), 3);
        st.doc.undo();
        let ObjectKind::Lettering(l) = &st.doc.design().objects()[0].kind else {
            panic!()
        };
        assert_eq!(l.text, "Anna");
        assert_eq!(summary(l), "Lato Bold · 20 mm · Anna");
    }
}
