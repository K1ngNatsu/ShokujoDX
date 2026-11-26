//! The Font Test Sheet window (P17 Milestone 4, ADR 0112): a new design
//! with the letters of a font at its smallest, middle and largest size, to
//! sew and to look at. The engine part is `emb_letter::sheet`.

use eframe::egui;
use emb_letter::sheet::{self, Characters, LABEL_FONT};
use emb_model::{FontRef, HoopSelection, LetterStitch};

use crate::app::AppState;
use crate::i18n::{fill, tr};

/// The space between the rows and the sides of the hoop, in mm.
const MARGIN_MM: f64 = 5.0;
/// The row width with no hoop, in mm.
const OPEN_WIDTH_MM: f64 = 180.0;
/// With no hoop, the sizes that are on at the start give a sheet that is
/// not higher than this, in mm.
const OPEN_HEIGHT_MM: f64 = 300.0;

/// The characters of the sheet (see `emb_letter::sheet::Characters`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CharSet {
    #[default]
    Basic,
    All,
    Capitals,
    Lowercase,
    Digits,
    Accented,
    Text,
}

impl CharSet {
    const ALL: [CharSet; 7] = [
        CharSet::Basic,
        CharSet::All,
        CharSet::Capitals,
        CharSet::Lowercase,
        CharSet::Digits,
        CharSet::Accented,
        CharSet::Text,
    ];

    fn label(self) -> &'static str {
        match self {
            CharSet::Basic => tr("Letters, digits and marks"),
            CharSet::All => tr("All characters of the font"),
            CharSet::Capitals => tr("Capital letters"),
            CharSet::Lowercase => tr("Small letters"),
            CharSet::Digits => tr("Digits and marks"),
            CharSet::Accented => tr("Letters with accents"),
            CharSet::Text => tr("My text"),
        }
    }

    fn characters(self, text: &str) -> Characters {
        match self {
            CharSet::Basic => Characters::Basic,
            CharSet::All => Characters::All,
            CharSet::Capitals => Characters::Capitals,
            CharSet::Lowercase => Characters::Lowercase,
            CharSet::Digits => Characters::Digits,
            CharSet::Accented => Characters::Accented,
            CharSet::Text => Characters::Text(text.to_string()),
        }
    }
}

/// The size of the sheet that the settings give.
#[derive(Debug, Clone, PartialEq)]
struct Info {
    width_mm: f64,
    height_mm: f64,
    letters: usize,
    /// The area of the group of each of the three sizes, alone (`None`: the
    /// font has no letters for it).
    groups: [Option<(f64, f64)>; 3],
}

#[derive(Debug, Clone)]
pub struct FontSheetWindow {
    pub open: bool,
    pub font: FontRef,
    /// The font that `sizes` and `range` are of.
    sized_for: Option<FontRef>,
    range: Option<(f64, f64)>,
    /// The smallest, the middle and the largest size: on, and the capital
    /// height in mm.
    pub sizes: [(bool, f64); 3],
    pub set: CharSet,
    pub text: String,
    /// The rows are not wider than this, in mm.
    pub width_mm: f64,
    /// The settings and what they give (made again after a change).
    info: Option<(String, Result<Info, String>)>,
}

impl Default for FontSheetWindow {
    fn default() -> Self {
        FontSheetWindow {
            open: false,
            font: FontRef::embroidery(emb_letter::bundled::DEFAULT_EMBROIDERY, ""),
            sized_for: None,
            range: None,
            sizes: [(true, 8.0), (true, 15.0), (true, 25.0)],
            set: CharSet::Basic,
            text: "Hamburg fonts 0123".into(),
            width_mm: OPEN_WIDTH_MM,
            info: None,
        }
    }
}

impl FontSheetWindow {
    /// The settings as a text: the sheet is made again when it changes.
    fn key(&self) -> String {
        format!(
            "{:?}|{:?}|{:?}|{}|{}",
            self.font, self.sizes, self.set, self.text, self.width_mm
        )
    }
}

impl AppState {
    /// The sewing field of the hoop of the design (width, height), in mm,
    /// if it has a hoop (the font test sheet and the test swatches).
    pub(crate) fn hoop_field(&self) -> Option<(f64, f64)> {
        match &self.doc.design().hoop.selection {
            HoopSelection::Preset { id } => self.hoops.get(id).map(|h| (h.width_mm, h.height_mm)),
            HoopSelection::OpenField => None,
        }
    }

    /// Opens the Font Test Sheet window for `font` (`None`: the font of
    /// new lettering), with rows that fit the hoop.
    pub fn open_font_sheet(&mut self, font: Option<FontRef>) {
        let width = self
            .hoop_field()
            .map_or(OPEN_WIDTH_MM, |(w, _)| (w - 2.0 * MARGIN_MM).max(20.0));
        let w = &mut self.font_sheet_window;
        w.font = font.unwrap_or_else(|| self.letter_font.clone());
        w.width_mm = width;
        w.open = true;
    }

    /// Gives the window the sizes of its font (after a change of the
    /// font). The smallest size is on; the middle and the largest size are
    /// on if the sheet with them fits the hoop.
    fn size_font_sheet(&mut self) {
        let font = self.font_sheet_window.font.clone();
        if self.font_sheet_window.sized_for.as_ref().is_some_and(|f| f.same(&font)) {
            return;
        }
        // The sheet of an embroidery font has the sizes of its columns: a
        // font with fill letters for large lettering goes to 300 mm.
        let stitch = match font.kind {
            emb_model::FontKind::Embroidery => LetterStitch::Satin,
            emb_model::FontKind::TrueType => LetterStitch::Auto,
        };
        let range = self.fonts.get(&font).and_then(|f| f.source().size_range(stitch));
        let s = sheet::sizes(range);
        let limit = self.hoop_field().map_or(OPEN_HEIGHT_MM, |(_, h)| h);
        let mut on = vec![s[0]];
        for size in [s[1], s[2]] {
            let mut with = on.clone();
            with.push(size);
            let fits = self
                .font_sheet_of(&with)
                .ok()
                .and_then(|made| made.bbox())
                .is_some_and(|b| b.height() <= limit);
            if fits {
                on = with;
            }
        }
        let w = &mut self.font_sheet_window;
        w.range = range;
        w.sizes = [(true, s[0]), (on.contains(&s[1]), s[1]), (on.contains(&s[2]), s[2])];
        w.sized_for = Some(font);
    }

    /// The test sheet of the settings of the window.
    pub fn font_sheet(&mut self) -> Result<sheet::Sheet, String> {
        let sizes: Vec<f64> = self
            .font_sheet_window
            .sizes
            .iter()
            .filter(|(on, _)| *on)
            .map(|(_, s)| *s)
            .collect();
        if sizes.is_empty() {
            return Err(tr("Select a size.").into());
        }
        self.font_sheet_of(&sizes)
    }

    /// The test sheet of the settings of the window, with these sizes.
    fn font_sheet_of(&mut self, sizes: &[f64]) -> Result<sheet::Sheet, String> {
        let w = self.font_sheet_window.clone();
        let font = self
            .fonts
            .get(&w.font)
            .ok_or(tr("This computer does not have the font."))?;
        let label = self.fonts.get(&FontRef::embroidery(LABEL_FONT, ""));
        let name = w.font.label();
        let characters = w.set.characters(&w.text);
        let spec = sheet::Spec {
            name: &name,
            sizes,
            characters: &characters,
            width_mm: w.width_mm,
        };
        let made = sheet::sheet(font.source(), label.as_ref().map(|l| l.source()), &spec);
        if made.groups.is_empty() {
            return Err(tr("The font has none of these characters.").into());
        }
        Ok(made)
    }

    /// Makes the test sheet as a new design, on the machine and in the
    /// hoop of the design before.
    pub fn make_font_sheet(&mut self) -> Result<(), String> {
        let made = self.font_sheet()?;
        let (letters, b) = (made.letters(), made.bbox());
        let name = self.font_sheet_window.font.label();
        let mut d = made.into_design(&fill("{} test sheet", &[&name]));
        d.machine = self.doc.design().machine.clone();
        d.hoop.selection = self.doc.design().hoop.selection.clone();
        self.start_design(d);
        let (w, h) = b.map_or((0.0, 0.0), |b| (b.width(), b.height()));
        self.message = fill(
            "Test sheet of {}: {} letters, {} × {} mm.",
            &[&name, &letters, &format!("{w:.0}"), &format!("{h:.0}")],
        );
        Ok(())
    }
}

/// The Font Test Sheet window.
pub fn window(ctx: &egui::Context, st: &mut AppState) {
    if !st.font_sheet_window.open {
        return;
    }
    st.size_font_sheet();
    // What the settings give, made again after a change.
    let key = st.font_sheet_window.key();
    if st.font_sheet_window.info.as_ref().is_none_or(|(k, _)| *k != key) {
        let sizes = st.font_sheet_window.sizes;
        // The area of each size alone, also of a size that is off.
        let groups = sizes.map(|(_, size)| {
            let made = st.font_sheet_of(&[size]).ok()?;
            made.bbox().map(|b| (b.width(), b.height()))
        });
        let info = st.font_sheet().map(|s| {
            let b = s.bbox();
            Info {
                width_mm: b.map_or(0.0, |b| b.width()),
                height_mm: b.map_or(0.0, |b| b.height()),
                letters: s.letters(),
                groups,
            }
        });
        st.font_sheet_window.info = Some((key, info));
    }
    let hoop = st.hoop_field();
    let mut w = st.font_sheet_window.clone();
    let info = w.info.as_ref().map(|(_, i)| i.clone());
    let (mut open, mut make) = (true, false);
    crate::dock::docked(
        egui::Window::new(tr("Font Test Sheet")).id(egui::Id::new("Font Test Sheet")),
        ctx,
        egui::Id::new("Font Test Sheet"),
        st.canvas_rect,
    )
    .open(&mut open)
    .collapsible(false)
    .default_width(400.0)
    .show(ctx, |ui| {
        ui.add(
            egui::Label::new(
                egui::RichText::new(tr(
                    "A new design with the letters of a font at some sizes. Sew it to see how the font \
                     sews. You can correct a letter in the Alphabets window.",
                ))
                .weak(),
            )
            .wrap(),
        );
        crate::lettering::font_picker(ui, st, &mut w.font, "sheet_font");
        if let Some((lo, hi)) = w.range {
            ui.weak(fill(
                "Size range: {}–{} mm (capital height)",
                &[&format!("{lo:.0}"), &format!("{hi:.0}")],
            ));
        }
        let groups = info.as_ref().and_then(|i| i.as_ref().ok()).map(|i| i.groups);
        egui::Grid::new("sheet_sizes").num_columns(3).show(ui, |ui| {
            for (i, name) in [tr("Smallest"), tr("Middle"), tr("Largest")].into_iter().enumerate() {
                let (on, size) = &mut w.sizes[i];
                ui.checkbox(on, name);
                ui.add_enabled(
                    *on,
                    crate::workspace::drag(size)
                        .range(2.0..=300.0)
                        .clamp_existing_to_range(false)
                        .speed(0.1)
                        .max_decimals(1)
                        .suffix(" mm"),
                );
                // The area of the letters of this size.
                match groups.and_then(|g| g[i]) {
                    Some((gw, gh)) => ui.weak(fill("{} × {} mm", &[&format!("{gw:.0}"), &format!("{gh:.0}")])),
                    None => ui.label(""),
                };
                ui.end_row();
            }
        });
        ui.horizontal(|ui| {
            ui.label(tr("Characters"));
            egui::ComboBox::from_id_salt("sheet_chars")
                .selected_text(w.set.label())
                .show_ui(ui, |ui| {
                    for s in CharSet::ALL {
                        ui.selectable_value(&mut w.set, s, s.label());
                    }
                });
        });
        if w.set == CharSet::Text {
            ui.add(
                egui::TextEdit::multiline(&mut w.text)
                    .desired_rows(2)
                    .desired_width(f32::INFINITY),
            );
        }
        ui.horizontal(|ui| {
            ui.label(tr("Row width"));
            ui.add(
                crate::workspace::drag(&mut w.width_mm)
                    .range(20.0..=1000.0)
                    .clamp_existing_to_range(false)
                    .speed(1.0)
                    .max_decimals(0)
                    .suffix(" mm"),
            )
            .on_hover_text(tr("The rows of letters are not wider than this."));
        });
        ui.separator();
        match &info {
            Some(Ok(i)) => {
                ui.label(fill(
                    "The sheet: {} × {} mm, {} letters.",
                    &[
                        &format!("{:.0}", i.width_mm),
                        &format!("{:.0}", i.height_mm),
                        &i.letters,
                    ],
                ));
                if let Some((hw, hh)) = hoop.filter(|(hw, hh)| i.width_mm > *hw || i.height_mm > *hh) {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(fill(
                                "The sheet is larger than the hoop ({} × {} mm). Use fewer sizes or \
                                 characters, or sew it in more than one hooping (the Hooping tab).",
                                &[&format!("{hw:.0}"), &format!("{hh:.0}")],
                            ))
                            .color(ui.visuals().warn_fg_color),
                        )
                        .wrap(),
                    );
                }
            }
            Some(Err(e)) => {
                ui.colored_label(ui.visuals().warn_fg_color, e);
            }
            None => {}
        }
        ui.horizontal(|ui| {
            if ui
                .add_enabled(matches!(info, Some(Ok(_))), egui::Button::new(tr("Make Test Sheet")))
                .on_hover_text(tr("The sheet opens as a new design."))
                .clicked()
            {
                make = true;
            }
        });
    });
    w.open = open;
    st.font_sheet_window = w;
    if make {
        match st.make_font_sheet() {
            Ok(()) => st.font_sheet_window.open = false,
            Err(e) => st.message = e,
        }
    }
}

#[cfg(test)]
mod tests {
    use emb_model::{HoopCatalog, MachineCatalog, ObjectKind};

    use super::*;

    fn state() -> AppState {
        AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new())
    }

    fn frame(ctx: &egui::Context, st: &mut AppState) {
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| window(ui.ctx(), st));
        out.textures_delta.clear();
    }

    #[test]
    fn the_window_gives_the_sizes_of_the_font_and_makes_the_sheet() {
        let mut st = state();
        st.open_font_sheet(None);
        let ctx = egui::Context::default();
        frame(&ctx, &mut st);
        frame(&ctx, &mut st);
        let w = st.font_sheet_window.clone();
        let font = emb_letter::bundled::embroidery(emb_letter::bundled::DEFAULT_EMBROIDERY).unwrap();
        assert_eq!(w.range, Some((font.min_height_mm, font.column_max_mm())));
        // The sizes of the font; the largest one gives a sheet that does
        // not fit the hoop, so it is off at the start.
        assert_eq!(w.sizes[0], (true, font.min_height_mm));
        assert_eq!(w.sizes[2], (false, font.column_max_mm()));
        let Some((_, Ok(info))) = &w.info else {
            panic!("the size of the sheet: {:?}", w.info)
        };
        assert!(info.groups.iter().all(Option::is_some), "the area of each size");
        assert!(info.width_mm <= w.width_mm + 1e-6);
        let (_, hoop_height) = st.hoop_field().expect("a hoop");
        assert!(info.height_mm <= hoop_height, "the sheet fits the hoop at the start");
        // The smallest size only: a design with a label and the letters.
        st.font_sheet_window.sizes[1].0 = false;
        st.make_font_sheet().unwrap();
        assert_eq!(st.tabs.len(), 1, "the empty design is used");
        let d = st.doc.design();
        assert!(d.name.contains("Block Sans"), "{}", d.name);
        assert_eq!(d.objects().len(), 2);
        let ObjectKind::Lettering(l) = &d.objects()[1].kind else {
            panic!("lettering")
        };
        assert_eq!(l.height_mm, font.min_height_mm);
        assert!(l.font.same(&w.font));
        assert!(st.message.contains("Block Sans"), "{}", st.message);
        assert!(st.plan_view().plan.stitches.stitch_count() > 2000);
        // A design with objects stays: the next sheet is a new tab.
        st.make_font_sheet().unwrap();
        assert_eq!(st.tabs.len(), 2);
    }

    #[test]
    fn settings_with_no_sheet_give_a_message() {
        let mut st = state();
        st.open_font_sheet(None);
        let ctx = egui::Context::default();
        frame(&ctx, &mut st);
        st.font_sheet_window.sizes = [(false, 8.0); 3];
        assert!(st.font_sheet().is_err());
        st.font_sheet_window.sizes[0].0 = true;
        st.font_sheet_window.set = CharSet::Text;
        st.font_sheet_window.text = " ".into();
        assert!(st.make_font_sheet().is_err());
        // The window shows the message and does not make a design.
        frame(&ctx, &mut st);
        assert!(matches!(st.font_sheet_window.info, Some((_, Err(_)))));
        // The text of the user.
        st.font_sheet_window.text = "Sewn".into();
        assert_eq!(st.font_sheet().unwrap().letters(), 4);
    }
}
