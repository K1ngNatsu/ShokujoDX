//! The Alphabets window (P4 Milestone 7, ADR 0021): the user's embroidery
//! fonts (alphabets) in the `fonts` folder next to the settings file.
//!
//! - Alphabet creator: guides for the baseline and the capital height on
//!   the canvas; draw a letter between them with the digitizing tools,
//!   select its objects, type the character and create the letter.
//! - Manage: new, rename, delete, import (an embroidery font, or a bitmap
//!   font: BDF or Windows `.fon`), export, delete a letter.

use std::path::{Path, PathBuf};

use eframe::egui;
use emb_letter::EmbFont;
use emb_letter::embfont::FILE_EXTENSION;
use emb_model::{FontRef, Guide, LetterStitch, ObjectKind};

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
            .ok_or("No folder for fonts.")?;
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        Ok(dir)
    }

    /// Saves `f` in the alphabets folder as `name` and loads it in the font
    /// lists. Fails if another file has the name.
    fn save_alphabet(&mut self, f: &EmbFont, old: Option<&Path>) -> Result<PathBuf, String> {
        let path = self.alphabets_dir()?.join(file_name(&f.family));
        if old != Some(path.as_path()) && path.exists() {
            return Err(format!("An alphabet named {} is there already.", f.family));
        }
        f.save(&path)?;
        self.fonts.add_user_font(&path)?;
        Ok(path)
    }

    /// A new alphabet with no letters. Returns its file.
    pub fn alphabet_new(&mut self, name: &str) -> Result<PathBuf, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Type a name for the alphabet.".into());
        }
        let mut f = EmbFont::empty(name, "");
        f.notes = "Made with the alphabet creator.".into();
        self.save_alphabet(&f, None)
    }

    pub fn alphabet_rename(&mut self, path: &Path, name: &str) -> Result<PathBuf, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Type the new name.".into());
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
        let letter = emb_letter::embfont::letter_from_objects(&kinds, w.baseline_y, w.height_mm)
            .ok_or("Select the satin, fill or running objects of the letter.")?;
        let mut f = EmbFont::load(path)?;
        f.set_letter(c, letter);
        f.save(path)?;
        self.fonts.add_user_font(path)?;
        Ok(())
    }

    pub fn alphabet_remove_letter(&mut self, path: &Path, c: char) -> Result<(), String> {
        let mut f = EmbFont::load(path)?;
        f.remove_letter(c);
        f.save(path)?;
        self.fonts.add_user_font(path)?;
        Ok(())
    }

    /// Guides at the baseline and at the capital height of the window.
    pub fn alphabet_guides(&mut self) {
        let (b, h) = (self.alphabets_window.baseline_y, self.alphabets_window.height_mm);
        self.doc.edit("Add letter guides", |d| {
            for pos in [b, b - h] {
                if !d.guides.iter().any(|g| !g.vertical && (g.pos - pos).abs() < 1e-6) {
                    d.guides.push(Guide { vertical: false, pos });
                }
            }
        });
    }
}

/// The Alphabets window.
pub fn window(ctx: &egui::Context, st: &mut AppState) {
    if !st.alphabets_window.open {
        return;
    }
    let list = st.alphabets_dir().map(|d| alphabets(&d)).unwrap_or_default();
    let mut w = st.alphabets_window.clone();
    if w.selected.as_ref().is_none_or(|p| !list.iter().any(|(q, _)| q == p)) {
        w.selected = list.first().map(|(p, _)| p.clone());
    }
    let font = w.selected.as_deref().and_then(|p| EmbFont::load(p).ok());
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
                        .map_or("(none yet)", |(_, n)| n.as_str()),
                )
                .show_ui(ui, |ui| {
                    for (p, n) in &list {
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
                    "Click again to delete"
                } else {
                    "Delete"
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
                    LetterStitch::Auto => "Bitmap fonts: automatic",
                    LetterStitch::Satin => "Bitmap fonts: satin",
                    LetterStitch::Fill => "Bitmap fonts: fill (pixel look)",
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
        if let Some(f) = &font {
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
            "New alphabet.".into()
        }),
        Act::Rename => match &sel {
            Some(p) => st.alphabet_rename(p, &w.rename).map(|n| {
                st.alphabets_window.selected = Some(n);
                st.alphabets_window.rename.clear();
                "Renamed.".into()
            }),
            None => Ok(String::new()),
        },
        Act::Delete => match &sel {
            Some(p) if w.confirm_delete => st.alphabet_delete(p).map(|()| {
                st.alphabets_window.confirm_delete = false;
                st.alphabets_window.selected = None;
                "Alphabet deleted.".into()
            }),
            Some(_) => {
                st.alphabets_window.confirm_delete = true;
                Ok("Click Delete again to delete the alphabet.".into())
            }
            None => Ok(String::new()),
        },
        Act::Import => match rfd::FileDialog::new()
            .set_title("Import an alphabet")
            .add_filter("Embroidery and bitmap fonts", &[FILE_EXTENSION, "bdf", "fon"])
            .set_directory(st.dialog_dir())
            .pick_file()
        {
            Some(src) => st.alphabet_import(&src, w.import_stitch).map(|p| {
                st.alphabets_window.selected = Some(p);
                "Imported.".into()
            }),
            None => Ok(String::new()),
        },
        Act::Export => match (&sel, sel.as_ref().and_then(|p| p.file_name()).map(|n| n.to_owned())) {
            (Some(p), Some(name)) => match rfd::FileDialog::new()
                .set_title("Export the alphabet")
                .set_file_name(name.to_string_lossy())
                .set_directory(st.dialog_dir())
                .save_file()
            {
                Some(dest) => st
                    .alphabet_export(p, &dest)
                    .map(|()| format!("Exported {}", dest.display())),
                None => Ok(String::new()),
            },
            _ => Ok(String::new()),
        },
        Act::Create => match (&sel, w.ch.chars().next()) {
            (Some(p), Some(c)) => st
                .alphabet_letter_from_selection(p, c)
                .map(|()| format!("\"{c}\" is now a letter of the alphabet.")),
            _ => Ok(String::new()),
        },
        Act::Guides => {
            st.alphabet_guides();
            Ok("Guides at the baseline and the capital height.".into())
        }
        Act::RemoveLetter(c) => match &sel {
            Some(p) => st.alphabet_remove_letter(p, c).map(|()| format!("\"{c}\" deleted.")),
            None => Ok(String::new()),
        },
        Act::LetterWith => match font {
            Some(f) => {
                st.letter_font = FontRef::embroidery(&f.family, &f.style);
                st.set_tool(crate::draw::Tool::Lettering);
                Ok(format!("Lettering with {}: click where the text starts.", f.family))
            }
            None => Ok(String::new()),
        },
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
}
