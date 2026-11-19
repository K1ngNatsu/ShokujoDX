//! The Alphabets window (P4 Milestone 7, ADR 0021): the user's embroidery
//! fonts (alphabets) in the `fonts` folder next to the settings file.
//!
//! - Alphabet creator: guides for the baseline and the capital height on
//!   the canvas; draw a letter between them with the digitizing tools,
//!   select its objects, type the character and create the letter.
//! - Manage: new, rename, delete, import (an embroidery font, or a bitmap
//!   font: BDF or Windows `.fon`), export, delete a letter.
//! - Letter corrections (P17 Milestone 4, ADR 0112): place a letter of an
//!   embroidery font as objects, change them, and save them as the
//!   correction of the letter. The corrections of a font are a file in the
//!   `corrections` folder of the fonts folder; the font library applies
//!   them when it reads the font.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use eframe::egui;
use emb_geom::Point;
use emb_letter::EmbFont;
use emb_letter::correction::{Corrections, LetterCorrection};
use emb_letter::embfont::{FILE_EXTENSION, LetterFrame};
use emb_model::{FontKind, FontRef, Guide, LetterStitch, ObjectKind};

use crate::a11y::Named;
use crate::app::AppState;
use crate::i18n::tr;
use crate::lettering::user_fonts_dir;

#[derive(Debug, Clone)]
pub struct AlphabetsWindow {
    pub open: bool,
    /// The file of the selected alphabet.
    pub selected: Option<PathBuf>,
    pub new_name: String,
    pub rename: String,
    /// Delete was clicked once (the second click deletes).
    pub confirm_delete: bool,
    /// The character for Create letter.
    pub ch: String,
    /// The capital height of the letters as drawn, in mm.
    pub height_mm: f64,
    /// The Y of the baseline of the letters as drawn (design mm).
    pub baseline_y: f64,
    /// The stitch type of imported bitmap fonts.
    pub import_stitch: LetterStitch,
    /// A letter keeps the sew order and the directions of its objects.
    pub keep_order: bool,
    /// A letter keeps the underlay, the pull compensation and the fill
    /// angle of its objects, where they are not the standard settings.
    pub keep_settings: bool,
    /// Letter corrections: the embroidery font and the character.
    pub fix_font: FontRef,
    pub fix_ch: String,
    /// Where Place Letter put the start of a letter: the font, the
    /// character and the X in the design.
    pub placed: Option<(FontRef, char, f64)>,
    /// The alphabets and the selected font, read when the window opens and
    /// after a change of the files (not on each frame).
    pub cache: Option<Arc<AlphabetsCache>>,
}

/// The files that the Alphabets window shows.
#[derive(Debug)]
pub struct AlphabetsCache {
    /// The user's alphabets: (file, family), by name.
    pub list: Vec<(PathBuf, String)>,
    /// The file that `font` is from.
    pub font_path: Option<PathBuf>,
    pub font: Option<EmbFont>,
    /// The font that `fixed` is of.
    pub fix_font: Option<FontRef>,
    /// The letters with a correction of the user.
    pub fixed: Vec<char>,
}

impl Default for AlphabetsWindow {
    fn default() -> Self {
        AlphabetsWindow {
            open: false,
            selected: None,
            new_name: String::new(),
            rename: String::new(),
            confirm_delete: false,
            ch: "A".into(),
            height_mm: 25.0,
            baseline_y: 12.5,
            import_stitch: LetterStitch::Auto,
            keep_order: true,
            keep_settings: false,
            fix_font: FontRef::embroidery(emb_letter::bundled::DEFAULT_EMBROIDERY, ""),
            fix_ch: "A".into(),
            placed: None,
            cache: None,
        }
    }
}

/// A file name from an alphabet name.
fn file_name(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || " -_".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!("{}.{FILE_EXTENSION}", s.trim())
}

/// The user's alphabets: (file, family), by name.
pub fn alphabets(dir: &Path) -> Vec<(PathBuf, String)> {
    let mut v: Vec<(PathBuf, String)> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some(FILE_EXTENSION))
        .filter_map(|p| {
            let f = EmbFont::load(&p).ok()?;
            Some((p, f.family))
        })
        .collect();
    v.sort_by_key(|(_, n)| n.to_lowercase());
    v
}

impl AppState {
    fn alphabets_dir(&self) -> Result<PathBuf, String> {
        let dir = self
            .alphabets_dir_override
            .clone()
            .or_else(user_fonts_dir)
            .ok_or(tr("No folder for fonts."))?;
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        Ok(dir)
    }

    /// Saves `f` in the alphabets folder as `name` and loads it in the font
    /// lists. Fails if another file has the name.
    fn save_alphabet(&mut self, f: &EmbFont, old: Option<&Path>) -> Result<PathBuf, String> {
        let path = self.alphabets_dir()?.join(file_name(&f.family));
        if old != Some(path.as_path()) && path.exists() {
            return Err(crate::i18n::fill(
                "An alphabet named {} is there already.",
                &[&f.family],
            ));
        }
        f.save(&path)?;
        self.fonts.add_user_font(&path)?;
        self.alphabets_window.cache = None;
        Ok(path)
    }

    /// A new alphabet with no letters. Returns its file.
    pub fn alphabet_new(&mut self, name: &str) -> Result<PathBuf, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err(tr("Type a name for the alphabet.").into());
        }
        let mut f = EmbFont::empty(name, "");
        f.notes = "Made with the alphabet creator.".into();
        self.save_alphabet(&f, None)
    }

    pub fn alphabet_rename(&mut self, path: &Path, name: &str) -> Result<PathBuf, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err(tr("Type the new name.").into());
        }
        let mut f = EmbFont::load(path)?;
        let old = FontRef::embroidery(&f.family, &f.style);
        f.family = name.to_string();
        let new = self.save_alphabet(&f, Some(path))?;
        if new != path {
            std::fs::remove_file(path).map_err(|e| format!("{}: {e}", path.display()))?;
        }
        self.fonts.forget(&old);
        Ok(new)
    }

    pub fn alphabet_delete(&mut self, path: &Path) -> Result<(), String> {
        let f = EmbFont::load(path)?;
        std::fs::remove_file(path).map_err(|e| format!("{}: {e}", path.display()))?;
        self.fonts.forget(&FontRef::embroidery(&f.family, &f.style));
        self.alphabets_window.cache = None;
        Ok(())
    }

    /// Adds an alphabet from a file: an embroidery font, or a bitmap font
    /// (the largest size) made into one.
    pub fn alphabet_import(&mut self, src: &Path, stitch: LetterStitch) -> Result<PathBuf, String> {
        let ext = src
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        let f = if ext == FILE_EXTENSION {
            EmbFont::load(src)?
        } else {
            let bytes = std::fs::read(src).map_err(|e| format!("{}: {e}", src.display()))?;
            let faces = emb_letter::bitmap::read_file(&ext, &bytes).map_err(|e| format!("{}: {e}", src.display()))?;
            let family = faces[0].family.clone();
            emb_letter::bitmap::to_embfont(&faces[0], &family, stitch)
        };
        self.save_alphabet(&f, None)
    }

    pub fn alphabet_export(&mut self, path: &Path, dest: &Path) -> Result<(), String> {
        std::fs::copy(path, dest).map_err(|e| format!("{}: {e}", dest.display()))?;
        Ok(())
    }

    /// Makes the selected objects letter `c` of the alphabet in `path`, with
    /// the baseline and the height of the window.
    pub fn alphabet_letter_from_selection(&mut self, path: &Path, c: char) -> Result<(), String> {
        let d = self.doc.design();
        let kinds: Vec<ObjectKind> = self
            .sel()
            .iter()
            .filter_map(|id| d.object(*id))
            .map(|o| o.kind.clone())
            .collect();
        let w = &self.alphabets_window;
        let frame = LetterFrame {
            baseline_y: w.baseline_y,
            height_mm: w.height_mm,
            origin_x: None,
            keep_order: w.keep_order,
            keep_settings: w.keep_settings,
        };
        let letter = emb_letter::embfont::letter_from(&kinds, &frame)
            .ok_or(tr("Select the satin, fill or running objects of the letter."))?;
        let mut f = EmbFont::load(path)?;
        f.set_letter(c, letter);
        f.save(path)?;
        self.fonts.add_user_font(path)?;
        self.alphabets_window.cache = None;
        Ok(())
    }

    pub fn alphabet_remove_letter(&mut self, path: &Path, c: char) -> Result<(), String> {
        let mut f = EmbFont::load(path)?;
        f.remove_letter(c);
        f.save(path)?;
        self.fonts.add_user_font(path)?;
        self.alphabets_window.cache = None;
        Ok(())
    }

    /// The folder of the letter corrections. The font library reads it
    /// from now on.
    fn corrections_dir(&mut self) -> Result<PathBuf, String> {
        let dir = self.alphabets_dir()?.join(crate::lettering::CORRECTIONS_DIR);
        self.fonts.corrections_dir = Some(dir.clone());
        Ok(dir)
    }

    /// The file of the letter corrections of `font`.
    pub fn corrections_file(&mut self, font: &FontRef) -> Result<PathBuf, String> {
        Ok(self
            .corrections_dir()?
            .join(emb_letter::correction::file_name(&font.family)))
    }

    /// The letters of `font` that have a correction of the user.
    pub fn corrected_letters(&mut self, font: &FontRef) -> Vec<char> {
        self.corrections_file(font)
            .ok()
            .filter(|p| p.is_file())
            .and_then(|p| Corrections::load(&p).ok())
            .map(|c| c.letters.keys().copied().collect())
            .unwrap_or_default()
    }

    fn embroidery_font(&mut self, font: &FontRef) -> Result<Arc<EmbFont>, String> {
        match self.fonts.get(font) {
            Some(emb_letter::AnyFont::Embroidery(f)) => Ok(f),
            _ => Err(tr("Select an embroidery font.").into()),
        }
    }

    /// Puts letter `c` of the embroidery font `font` in the design as
    /// objects, in sew order, with the height and the baseline of the
    /// window, between guides at the start and the end of the letter. The
    /// objects are selected.
    pub fn place_letter(&mut self, font: &FontRef, c: char) -> Result<(), String> {
        let f = self.embroidery_font(font)?;
        let letter = f
            .letters
            .get(&c)
            .filter(|l| !l.parts.is_empty())
            .ok_or_else(|| crate::i18n::fill("{} has no letter \"{}\".", &[&f.family, &c]))?;
        let (h, base) = (self.alphabets_window.height_mm, self.alphabets_window.baseline_y);
        let width = letter.advance_mm * h / f.cap_height_mm.max(1e-6);
        let x0 = -width / 2.0;
        let kinds = emb_letter::embfont::objects_of_letter(&f, letter, Point::new(x0, base), h);
        let current = self.current_thread;
        let ids = self.doc.edit(tr("Place letter"), |d| {
            for (vertical, pos) in [(false, base), (false, base - h), (true, x0), (true, x0 + width)] {
                if !d
                    .guides
                    .iter()
                    .any(|g| g.vertical == vertical && (g.pos - pos).abs() < 1e-6)
                {
                    d.guides.push(Guide::new(vertical, pos));
                }
            }
            let thread = current
                .filter(|t| d.thread(*t).is_some())
                .or(d.threads.first().map(|t| t.id))
                .unwrap_or_else(|| d.add_thread("Black", emb_model::Rgb(0, 0, 0)));
            kinds.into_iter().map(|k| d.add_object(thread, k)).collect::<Vec<_>>()
        });
        self.selection.set_all(ids);
        self.fit_requested = true;
        let w = &mut self.alphabets_window;
        w.placed = Some((font.clone(), c, x0));
        // The objects have the standard settings and the settings of the
        // letter, so a change of them is a change of the letter.
        w.keep_settings = true;
        Ok(())
    }

    /// Makes the selected objects the correction of letter `c` of `font`,
    /// with the baseline and the height of the window. A letter that Place
    /// Letter put there keeps its start and its width.
    pub fn save_correction(&mut self, font: &FontRef, c: char) -> Result<(), String> {
        let f = self.embroidery_font(font)?;
        let d = self.doc.design();
        let kinds: Vec<ObjectKind> = self
            .sel()
            .iter()
            .filter_map(|id| d.object(*id))
            .map(|o| o.kind.clone())
            .collect();
        let w = &self.alphabets_window;
        let origin_x = w
            .placed
            .as_ref()
            .filter(|(pf, pc, _)| pf.same(font) && *pc == c)
            .map(|p| p.2);
        let frame = LetterFrame {
            baseline_y: w.baseline_y,
            height_mm: w.height_mm,
            origin_x,
            keep_order: w.keep_order,
            keep_settings: w.keep_settings,
        };
        let mut letter = emb_letter::embfont::letter_from(&kinds, &frame)
            .ok_or(tr("Select the satin, fill or running objects of the letter."))?;
        // The letters of a correction are at the design size of the font.
        let k = f.cap_height_mm / emb_letter::embfont::DESIGN_CAP_MM;
        if (k - 1.0).abs() > 1e-9 {
            for p in &mut letter.parts {
                p.transform(&emb_geom::Transform::scale(k, k));
            }
            letter.advance_mm *= k;
        }
        if origin_x.is_some()
            && let Some(old) = f.letters.get(&c)
        {
            letter.advance_mm = old.advance_mm;
        }
        for p in &mut letter.parts {
            emb_letter::embfont::round_part(p);
        }
        letter.advance_mm = (letter.advance_mm * 100.0).round() / 100.0;
        let path = self.corrections_file(font)?;
        let mut all = if path.is_file() {
            Corrections::load(&path)?
        } else {
            Corrections::new(&f.family)
        };
        all.family = f.family.clone();
        all.letters.insert(c, LetterCorrection::from_letter(&letter));
        all.save(&path)?;
        self.fonts.reload(font);
        self.alphabets_window.cache = None;
        Ok(())
    }

    /// Removes the correction of letter `c` of `font`. Returns `false` if
    /// the letter has none.
    pub fn remove_correction(&mut self, font: &FontRef, c: char) -> Result<bool, String> {
        let path = self.corrections_file(font)?;
        if !path.is_file() {
            return Ok(false);
        }
        let mut all = Corrections::load(&path)?;
        if all.letters.remove(&c).is_none() {
            return Ok(false);
        }
        if all.letters.is_empty() {
            std::fs::remove_file(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        } else {
            all.save(&path)?;
        }
        self.fonts.reload(font);
        self.alphabets_window.cache = None;
        Ok(true)
    }

    /// Guides at the baseline and at the capital height of the window.
    pub fn alphabet_guides(&mut self) {
        let (b, h) = (self.alphabets_window.baseline_y, self.alphabets_window.height_mm);
        self.doc.edit(tr("Add letter guides"), |d| {
            for pos in [b, b - h] {
                if !d.guides.iter().any(|g| !g.vertical && (g.pos - pos).abs() < 1e-6) {
                    d.guides.push(Guide::new(false, pos));
                }
            }
        });
    }
}

/// The Alphabets window.
pub fn window(ctx: &egui::Context, st: &mut AppState) {
    if !st.alphabets_window.open {
        st.alphabets_window.cache = None;
        return;
    }
    let cache = match st.alphabets_window.cache.clone() {
        Some(c) => c,
        None => {
            let list = st.alphabets_dir().map(|d| alphabets(&d)).unwrap_or_default();
            Arc::new(AlphabetsCache {
                list,
                font_path: None,
                font: None,
                fix_font: None,
                fixed: Vec::new(),
            })
        }
    };
    let list = &cache.list;
    let mut w = st.alphabets_window.clone();
    if w.selected.as_ref().is_none_or(|p| !list.iter().any(|(q, _)| q == p)) {
        w.selected = list.first().map(|(p, _)| p.clone());
    }
    // The selected font: loaded again only when the selection changes.
    let cache = if cache.font_path != w.selected {
        Arc::new(AlphabetsCache {
            list: cache.list.clone(),
            font_path: w.selected.clone(),
            font: w.selected.as_deref().and_then(|p| EmbFont::load(p).ok()),
            fix_font: cache.fix_font.clone(),
            fixed: cache.fixed.clone(),
        })
    } else {
        cache
    };
    // The corrected letters: read again only for another font.
    let cache = if cache.fix_font.as_ref().is_none_or(|f| !f.same(&w.fix_font)) {
        Arc::new(AlphabetsCache {
            list: cache.list.clone(),
            font_path: cache.font_path.clone(),
            font: cache.font.clone(),
            fix_font: Some(w.fix_font.clone()),
            fixed: st.corrected_letters(&w.fix_font),
        })
    } else {
        cache
    };
    w.cache = Some(cache.clone());
    let list = &cache.list;
    let font = cache.font.as_ref();
    let fixed = &cache.fixed;
    let fix_fonts: Vec<FontRef> = st
        .fonts
        .entries
        .iter()
        .filter(|e| e.font.kind == FontKind::Embroidery)
        .map(|e| e.font.clone())
        .collect();
    let mut open = true;
    #[derive(PartialEq)]
    enum Act {
        None,
        New,
        Rename,
        Delete,
        Import,
        Export,
        Create,
        Guides,
        RemoveLetter(char),
        LetterWith,
        Place,
        SaveFix,
        RemoveFix,
        ShowFixes,
    }
    let mut act = Act::None;
    crate::dock::docked(
        egui::Window::new(tr("Alphabets")).id(egui::Id::new("Alphabets")),
        ctx,
        egui::Id::new("Alphabets"),
        st.canvas_rect,
    )
    .open(&mut open)
    .collapsible(false)
    .default_width(420.0)
    .show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.label(tr("Alphabet"));
            egui::ComboBox::from_id_salt("alphabet")
                .selected_text(
                    w.selected
                        .as_ref()
                        .and_then(|p| list.iter().find(|(q, _)| q == p))
                        .map_or(tr("(none yet)"), |(_, n)| n.as_str()),
                )
                .show_ui(ui, |ui| {
                    for (p, n) in list {
                        ui.selectable_value(&mut w.selected, Some(p.clone()), n);
                    }
                });
        });
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut w.new_name)
                    .hint_text(tr("Name"))
                    .desired_width(140.0),
            );
            if ui.button(tr("New alphabet")).clicked() {
                act = Act::New;
            }
        });
        let has = w.selected.is_some();
        ui.add_enabled_ui(has, |ui| {
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut w.rename)
                        .hint_text(tr("New name"))
                        .desired_width(140.0),
                );
                if ui.button(tr("Rename")).clicked() {
                    act = Act::Rename;
                }
                let del = if w.confirm_delete {
                    tr("Click again to delete")
                } else {
                    tr("Delete")
                };
                if ui
                    .button(del)
                    .on_hover_text(tr("Delete the alphabet file."))
                    .named(tr("Delete the alphabet file."))
                    .clicked()
                {
                    act = Act::Delete;
                }
            });
        });
        ui.horizontal(|ui| {
            if ui
                .button(tr("Import…"))
                .on_hover_text(tr(
                    "An embroidery font (.embfont), or a bitmap font (.bdf, .fon) made into one.",
                ))
                .clicked()
            {
                act = Act::Import;
            }
            egui::ComboBox::from_id_salt("import_stitch")
                .selected_text(match w.import_stitch {
                    LetterStitch::Auto => tr("Bitmap fonts: automatic"),
                    LetterStitch::Satin => tr("Bitmap fonts: satin"),
                    LetterStitch::Fill => tr("Bitmap fonts: fill (pixel look)"),
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut w.import_stitch, LetterStitch::Auto, tr("Automatic"));
                    ui.selectable_value(&mut w.import_stitch, LetterStitch::Satin, tr("Satin"));
                    ui.selectable_value(&mut w.import_stitch, LetterStitch::Fill, tr("Fill (pixel look)"));
                });
            if ui.add_enabled(has, egui::Button::new(tr("Export…"))).clicked() {
                act = Act::Export;
            }
        });
        if let Some(f) = font {
            ui.separator();
            ui.horizontal(|ui| {
                ui.strong(crate::i18n::fill(
                    "Letters ({})",
                    &[&(f.letters.iter().filter(|(_, l)| !l.parts.is_empty()).count())],
                ));
                if ui.button(tr("Letter with this alphabet")).clicked() {
                    act = Act::LetterWith;
                }
            });
            ui.horizontal_wrapped(|ui| {
                for (c, l) in &f.letters {
                    if l.parts.is_empty() {
                        continue;
                    }
                    if ui
                        .small_button(c.to_string())
                        .on_hover_text(tr("Click: delete this letter."))
                        .clicked()
                    {
                        act = Act::RemoveLetter(*c);
                    }
                }
            });
        }
        ui.separator();
        ui.strong(tr("Alphabet creator"));
        ui.weak(tr(
            "Add the guides, draw the letter between them with the digitizing tools, \
                 select its objects, type its character and click Create letter.",
        ));
        ui.horizontal(|ui| {
            ui.label(tr("Letter height"));
            ui.add(
                egui::DragValue::new(&mut w.height_mm)
                    .range(5.0..=200.0)
                    .clamp_existing_to_range(false)
                    .speed(0.2)
                    .suffix(" mm"),
            );
            ui.label(tr("Baseline at"));
            ui.add(egui::DragValue::new(&mut w.baseline_y).speed(0.2).suffix(" mm"));
            if ui.button(tr("Add guides")).clicked() {
                act = Act::Guides;
            }
        });
        ui.horizontal(|ui| {
            ui.label(tr("Character"));
            ui.add(egui::TextEdit::singleline(&mut w.ch).desired_width(30.0).char_limit(1));
            if ui
                .add_enabled(has && !w.ch.is_empty(), egui::Button::new(tr("Create letter")))
                .on_hover_text(tr("The selected objects become this letter of the alphabet."))
                .clicked()
            {
                act = Act::Create;
            }
        });
        ui.horizontal(|ui| {
            ui.checkbox(&mut w.keep_order, tr("Keep the sew order"))
                .on_hover_text(tr(
                    "The letter sews its objects in their order, each column from its start to its end. \
                 Off: the program finds the order.",
                ));
            ui.checkbox(&mut w.keep_settings, tr("Keep the settings"))
                .on_hover_text(tr(
                    "The letter keeps the underlay, the pull compensation and the fill angle of each \
                 object, where they are not the standard settings of lettering.",
                ));
        });
        ui.separator();
        ui.strong(tr("Letter corrections"));
        ui.add(
            egui::Label::new(
                egui::RichText::new(tr(
                    "Place a letter of an embroidery font, change its objects (the order, the \
                     direction, the shape, the settings), select them and save the correction. A new \
                     version of the font keeps your corrections.",
                ))
                .weak(),
            )
            .wrap(),
        );
        ui.horizontal(|ui| {
            ui.label(tr("Font"));
            egui::ComboBox::from_id_salt("fix_font")
                .selected_text(w.fix_font.label())
                .show_ui(ui, |ui| {
                    for f in &fix_fonts {
                        ui.selectable_value(&mut w.fix_font, f.clone(), f.label());
                    }
                });
            ui.label(tr("Character"));
            ui.add(
                egui::TextEdit::singleline(&mut w.fix_ch)
                    .desired_width(30.0)
                    .char_limit(1),
            );
        });
        let ch = w.fix_ch.chars().next();
        ui.horizontal(|ui| {
            if ui
                .add_enabled(ch.is_some(), egui::Button::new(tr("Place Letter")))
                .on_hover_text(tr(
                    "Put the objects of this letter in the design, between guides, in their sew order.",
                ))
                .clicked()
            {
                act = Act::Place;
            }
            if ui
                .add_enabled(ch.is_some(), egui::Button::new(tr("Save Correction")))
                .on_hover_text(tr("The selected objects become this letter of the font."))
                .clicked()
            {
                act = Act::SaveFix;
            }
            if ui
                .add_enabled(
                    ch.is_some_and(|c| fixed.contains(&c)),
                    egui::Button::new(tr("Remove Correction")),
                )
                .on_hover_text(tr("The letter is as the font has it again."))
                .clicked()
            {
                act = Act::RemoveFix;
            }
        });
        if !fixed.is_empty() {
            ui.horizontal_wrapped(|ui| {
                ui.label(tr("Corrected letters:"));
                for c in fixed {
                    if ui
                        .small_button(c.to_string())
                        .on_hover_text(tr("Click: this character."))
                        .clicked()
                    {
                        w.fix_ch = c.to_string();
                    }
                }
                if ui
                    .small_button(tr("Show File"))
                    .on_hover_text(tr("Show the file of the corrections of this font in its folder."))
                    .clicked()
                {
                    act = Act::ShowFixes;
                }
            });
        }
    });
    if act != Act::Delete {
        w.confirm_delete = false;
    }
    w.open = open;
    st.alphabets_window = w.clone();
    let sel = w.selected.clone();
    let result: Result<String, String> = match act {
        Act::None => return,
        Act::New => st.alphabet_new(&w.new_name).map(|p| {
            st.alphabets_window.selected = Some(p);
            st.alphabets_window.new_name.clear();
            tr("New alphabet.").into()
        }),
        Act::Rename => match &sel {
            Some(p) => st.alphabet_rename(p, &w.rename).map(|n| {
                st.alphabets_window.selected = Some(n);
                st.alphabets_window.rename.clear();
                tr("Renamed.").into()
            }),
            None => Ok(String::new()),
        },
        Act::Delete => match &sel {
            Some(p) if w.confirm_delete => st.alphabet_delete(p).map(|()| {
                st.alphabets_window.confirm_delete = false;
                st.alphabets_window.selected = None;
                tr("Alphabet deleted.").into()
            }),
            Some(_) => {
                st.alphabets_window.confirm_delete = true;
                Ok(tr("Click Delete again to delete the alphabet.").into())
            }
            None => Ok(String::new()),
        },
        Act::Import => match rfd::FileDialog::new()
            .set_title(tr("Import an alphabet"))
            .add_filter(tr("Embroidery and bitmap fonts"), &[FILE_EXTENSION, "bdf", "fon"])
            .set_directory(st.dialog_dir())
            .pick_file()
        {
            Some(src) => st.alphabet_import(&src, w.import_stitch).map(|p| {
                st.alphabets_window.selected = Some(p);
                tr("Imported.").into()
            }),
            None => Ok(String::new()),
        },
        Act::Export => match (&sel, sel.as_ref().and_then(|p| p.file_name()).map(|n| n.to_owned())) {
            (Some(p), Some(name)) => match rfd::FileDialog::new()
                .set_title(tr("Export the alphabet"))
                .set_file_name(name.to_string_lossy())
                .set_directory(st.dialog_dir())
                .save_file()
            {
                Some(dest) => st
                    .alphabet_export(p, &dest)
                    .map(|()| crate::i18n::fill("Exported {}", &[&dest.display()])),
                None => Ok(String::new()),
            },
            _ => Ok(String::new()),
        },
        Act::Create => match (&sel, w.ch.chars().next()) {
            (Some(p), Some(c)) => st
                .alphabet_letter_from_selection(p, c)
                .map(|()| crate::i18n::fill("\"{}\" is now a letter of the alphabet.", &[&c])),
            _ => Ok(String::new()),
        },
        Act::Guides => {
            st.alphabet_guides();
            Ok(tr("Guides at the baseline and the capital height.").into())
        }
        Act::RemoveLetter(c) => match &sel {
            Some(p) => st
                .alphabet_remove_letter(p, c)
                .map(|()| crate::i18n::fill("\"{}\" deleted.", &[&c])),
            None => Ok(String::new()),
        },
        Act::LetterWith => match font {
            Some(f) => {
                st.letter_font = FontRef::embroidery(&f.family, &f.style);
                st.set_tool(crate::draw::Tool::Lettering);
                Ok(crate::i18n::fill(
                    "Lettering with {}: click where the text starts.",
                    &[&f.family],
                ))
            }
            None => Ok(String::new()),
        },
        Act::Place => match w.fix_ch.chars().next() {
            Some(c) => st.place_letter(&w.fix_font, c).map(|()| {
                crate::i18n::fill(
                    "\"{}\" of {} is in the design. Change its objects, select them and save the correction.",
                    &[&c, &w.fix_font.label()],
                )
            }),
            None => Ok(String::new()),
        },
        Act::SaveFix => match w.fix_ch.chars().next() {
            Some(c) => st.save_correction(&w.fix_font, c).map(|()| {
                crate::i18n::fill(
                    "\"{}\" of {} is corrected. New lettering uses it.",
                    &[&c, &w.fix_font.label()],
                )
            }),
            None => Ok(String::new()),
        },
        Act::RemoveFix => match w.fix_ch.chars().next() {
            Some(c) => st.remove_correction(&w.fix_font, c).map(|removed| {
                if removed {
                    crate::i18n::fill("\"{}\" is as the font has it again.", &[&c])
                } else {
                    String::new()
                }
            }),
            None => Ok(String::new()),
        },
        Act::ShowFixes => st.corrections_file(&w.fix_font).and_then(|p| {
            crate::share::show_in_folder(&p)
                .map(|()| String::new())
                .map_err(|e| format!("{}: {e}", p.display()))
        }),
    };
    match result {
        Ok(m) if !m.is_empty() => st.message = m,
        Ok(_) => {}
        Err(e) => st.message = e,
    }
}

#[cfg(test)]
mod tests {
    use emb_geom::{BBox, Path, Point};
    use emb_model::{HoopCatalog, MachineCatalog};

    use super::*;
    use crate::lettering::LetteringDialog;

    fn state(name: &str) -> (AppState, PathBuf) {
        let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
        let dir = std::env::temp_dir().join(format!("emb-alpha-{name}-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        st.alphabets_dir_override = Some(dir.clone());
        (st, dir)
    }

    /// Draws `rects` (x0, y0, x1, y1) as fills and selects them.
    fn draw(st: &mut AppState, rects: &[(f64, f64, f64, f64)]) {
        let ids = st.doc.edit("Draw", |d| {
            let t = d.add_thread("Red", emb_model::Rgb(200, 0, 0));
            rects
                .iter()
                .map(|&(x0, y0, x1, y1)| {
                    d.add_object(
                        t,
                        ObjectKind::Fill {
                            outer: Path::rect(BBox::new(Point::new(x0, y0), Point::new(x1, y1))),
                            holes: vec![],
                            params: Default::default(),
                        },
                    )
                })
                .collect::<Vec<_>>()
        });
        st.selection.set_all(ids);
    }

    #[test]
    fn make_a_three_letter_alphabet_and_letter_with_it() {
        let (mut st, dir) = state("abc");
        let path = st.alphabet_new("My Blocks").unwrap();
        st.alphabets_window.baseline_y = 0.0;
        st.alphabets_window.height_mm = 20.0;
        // "I": one bar; "L": two bars; "T": two bars. Drawn 20 mm high.
        draw(&mut st, &[(0.0, -20.0, 4.0, 0.0)]);
        st.alphabet_letter_from_selection(&path, 'I').unwrap();
        draw(&mut st, &[(0.0, -20.0, 4.0, 0.0), (4.0, -4.0, 14.0, 0.0)]);
        st.alphabet_letter_from_selection(&path, 'L').unwrap();
        draw(&mut st, &[(0.0, -20.0, 14.0, -16.0), (5.0, -16.0, 9.0, 0.0)]);
        st.alphabet_letter_from_selection(&path, 'T').unwrap();
        let f = EmbFont::load(&path).unwrap();
        assert_eq!(f.letters.len(), 4, "three letters and the space");
        // Letter with it: it is in the font lists.
        st.doc.edit("Clear", |d| {
            let ids: Vec<_> = d.objects().iter().map(|o| o.id).collect();
            for id in ids {
                d.remove_object(id);
            }
        });
        st.add_lettering(LetteringDialog::new(
            Point::new(0.0, 0.0),
            "LIT",
            FontRef::embroidery("My Blocks", ""),
            40.0,
        ));
        let ObjectKind::Lettering(l) = &st.doc.design().objects()[0].kind else {
            panic!()
        };
        assert!(l.font.same(&FontRef::embroidery("My Blocks", "")));
        assert_eq!(l.letters.len(), 3);
        let b = l.parts().filter_map(|p| p.control_bbox()).reduce(BBox::union).unwrap();
        assert!((b.height() - 40.0).abs() < 0.1, "the capital height: {b:?}");
        assert!(st.plan_view().plan.stitches.stitch_count() > 100);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn correct_a_letter_of_a_bundled_font() {
        let (mut st, dir) = state("fix");
        // A font of the program: its letters are routed (ADR 0116), with
        // no settings of their own. The first part of this letter is a
        // column.
        let font = FontRef::embroidery("Anton", "");
        let before = st.embroidery_font(&font).unwrap().letters[&'H'].clone();
        let n = before.parts.len();
        assert!(n >= 3 && before.fixed && before.tuning.is_empty());
        assert!(matches!(before.parts[0], ObjectKind::Satin { .. }));
        // Place the letter: its objects are in the design, selected,
        // between four guides.
        st.alphabets_window.height_mm = 40.0;
        st.alphabets_window.baseline_y = 10.0;
        st.place_letter(&font, 'H').unwrap();
        let ids = st.sel();
        assert_eq!(ids.len(), n);
        assert_eq!(st.doc.design().guides.len(), 4);
        let area = |st: &AppState| {
            st.sel()
                .iter()
                .filter_map(|id| st.doc.design().object(*id))
                .filter_map(|o| o.kind.control_bbox())
                .reduce(BBox::union)
                .unwrap()
        };
        let b = area(&st);
        assert!((b.max.y - 10.0).abs() < 0.5 && (b.height() - 40.0).abs() < 0.5, "{b:?}");
        assert!(
            st.alphabets_window.keep_settings,
            "the settings of the objects are kept"
        );
        // A save with no change gives the letter of the font, in a fixed order.
        st.save_correction(&font, 'H').unwrap();
        let same = st.embroidery_font(&font).unwrap().letters[&'H'].clone();
        assert!(same.fixed && same.tuning.is_empty());
        assert_eq!(same.advance_mm, before.advance_mm);
        let boxes =
            |l: &emb_letter::EmbLetter| -> Vec<BBox> { l.parts.iter().filter_map(|p| p.control_bbox()).collect() };
        for (x, y) in boxes(&same).iter().zip(boxes(&before)) {
            assert!(
                x.min.distance(y.min) < 0.02 && x.max.distance(y.max) < 0.02,
                "{x:?} {y:?}"
            );
        }
        // Change the objects: the first one sews last, the other way, with
        // a heavy underlay and more pull compensation.
        let first = ids[0];
        st.doc.edit("Change", |d| {
            d.reverse_objects(&[first]);
            let o = d.object(first).unwrap().clone();
            let mut kind = o.kind.clone();
            if let ObjectKind::Satin { params, .. } = &mut kind {
                params.underlay = emb_model::Underlay::Heavy;
                params.pull_compensation_mm += 0.1;
            }
            d.remove_object(first);
            d.add_object(o.thread, kind);
        });
        let all: Vec<_> = st.doc.design().objects().iter().map(|o| o.id).collect();
        st.selection.set_all(all);
        st.save_correction(&font, 'H').unwrap();
        let file = dir.join("corrections").join("anton.json");
        assert!(file.is_file());
        assert_eq!(st.corrected_letters(&font), vec!['H']);
        let after = st.embroidery_font(&font).unwrap().letters[&'H'].clone();
        assert!(after.fixed);
        assert_eq!(after.parts.len(), n);
        let (a, b) = (boxes(&after), boxes(&before));
        assert!(a[0].min.distance(b[1].min) < 0.02, "the second part is the first now");
        assert!(a[n - 1].min.distance(b[0].min) < 0.02, "the first part is the last now");
        assert_eq!(after.tuning.len(), n);
        assert_eq!(after.tuning[n - 1].underlay, Some(emb_model::Underlay::Heavy));
        assert!((after.tuning[n - 1].pull_mm - 0.1).abs() < 1e-9);
        assert!(after.tuning[0].is_empty());
        // New lettering has the corrected letter.
        st.doc.edit("Clear", |d| {
            let ids: Vec<_> = d.objects().iter().map(|o| o.id).collect();
            for id in ids {
                d.remove_object(id);
            }
        });
        st.add_lettering(LetteringDialog::new(Point::new(0.0, 0.0), "HI", font.clone(), 20.0));
        let ObjectKind::Lettering(l) = &st.doc.design().objects()[0].kind else {
            panic!()
        };
        assert!(l.letters[0].fixed);
        let ObjectKind::Satin { params, .. } = &l.letters[0].parts[n - 1] else {
            panic!("a column")
        };
        assert_eq!(params.underlay, emb_model::Underlay::Heavy);
        assert!((params.pull_compensation_mm - l.satin.pull_compensation_mm - 0.1).abs() < 1e-9);
        // The window shows the corrected letters.
        st.alphabets_window.fix_font = font.clone();
        st.alphabets_window.open = true;
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| window(ui.ctx(), &mut st));
        out.textures_delta.clear();
        assert_eq!(st.alphabets_window.cache.as_ref().unwrap().fixed, vec!['H']);
        // With no correction, the letter is the letter of the font again.
        assert!(st.remove_correction(&font, 'H').unwrap());
        assert!(!st.remove_correction(&font, 'H').unwrap());
        assert!(!file.exists());
        assert_eq!(st.embroidery_font(&font).unwrap().letters[&'H'], before);
        // A font with no such letter, and a font that is not an embroidery font.
        assert!(st.place_letter(&font, '\u{4e00}').is_err());
        assert!(st.place_letter(&FontRef::new("Lato", "Bold"), 'H').is_err());
        st.selection.clear();
        assert!(st.save_correction(&font, 'H').is_err());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn manage_alphabets() {
        let (mut st, dir) = state("manage");
        let a = st.alphabet_new("One").unwrap();
        assert!(st.alphabet_new("One").is_err(), "names are unique");
        let b = st.alphabet_rename(&a, "Two").unwrap();
        assert!(!a.exists() && b.exists());
        assert!(st.fonts.get(&FontRef::embroidery("Two", "")).is_some());
        assert!(st.fonts.get(&FontRef::embroidery("One", "")).is_none());
        let out = dir.join("copy.out");
        st.alphabet_export(&b, &out).unwrap();
        assert!(out.exists());
        st.alphabet_delete(&b).unwrap();
        assert!(!b.exists());
        assert!(st.fonts.get(&FontRef::embroidery("Two", "")).is_none());
        // Guides for the creator.
        st.alphabet_guides();
        assert_eq!(st.doc.design().guides.len(), 2);
        assert_eq!(alphabets(&dir).len(), 0);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn the_window_draws() {
        let (mut st, dir) = state("draw");
        st.alphabet_new("Shown").unwrap();
        st.alphabets_window.open = true;
        let ctx = egui::Context::default();
        for _ in 0..2 {
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| window(ui.ctx(), &mut st));
            out.textures_delta.clear();
        }
        assert!(st.alphabets_window.selected.is_some());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn the_window_reads_the_files_only_after_a_change() {
        let (mut st, dir) = state("cache");
        let one = st.alphabet_new("One").unwrap();
        st.alphabets_window.open = true;
        let ctx = egui::Context::default();
        let frame = |st: &mut AppState| {
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| window(ui.ctx(), st));
            out.textures_delta.clear();
        };
        frame(&mut st);
        let first = st.alphabets_window.cache.clone().expect("read on the first frame");
        assert_eq!(first.font.as_ref().map(|f| f.family.as_str()), Some("One"));
        frame(&mut st);
        assert!(
            Arc::ptr_eq(&first, st.alphabets_window.cache.as_ref().unwrap()),
            "not read again"
        );
        // A new alphabet: the list is read again, with it.
        let two = st.alphabet_new("Two").unwrap();
        assert!(st.alphabets_window.cache.is_none());
        frame(&mut st);
        assert_eq!(st.alphabets_window.cache.as_ref().unwrap().list.len(), 2);
        // Another selection loads its font.
        st.alphabets_window.selected = Some(two);
        frame(&mut st);
        let c = st.alphabets_window.cache.clone().unwrap();
        assert_eq!(c.font.as_ref().map(|f| f.family.as_str()), Some("Two"));
        st.alphabet_delete(&one).unwrap();
        assert!(st.alphabets_window.cache.is_none());
        std::fs::remove_dir_all(dir).ok();
    }
}
