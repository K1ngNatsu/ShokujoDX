//! The Fonts window (P4 Milestone 2): find a font, see a preview and its
//! size range, use it for the selected lettering, and convert a TrueType
//! font to an embroidery font (ADR 0016).

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, channel};

use eframe::egui;
use egui::{Color32, Pos2, Rect, Shape, Vec2};
use emb_geom::{BBox, Point};
use emb_model::{FontKind, FontRef, Letter, LetterStitch, Lettering, ObjectKind};

use crate::app::AppState;
use crate::i18n::tr;
use crate::lettering::{kind_name, relayout};

/// The text of the preview.
pub const PREVIEW_TEXT: &str = "AaBbCc Gg 123";

/// A font made ready for the preview.
#[derive(Debug, Clone)]
struct Preview {
    font: FontRef,
    letters: Vec<Letter>,
    range: Option<(f64, f64)>,
    notes: String,
}

/// The form to convert a TrueType font.
#[derive(Debug, Clone)]
struct ConvertForm {
    source: FontRef,
    name: String,
    stitch: LetterStitch,
}

#[derive(Debug, Default)]
pub struct FontsWindow {
    pub open: bool,
    filter: String,
    /// Show one kind only.
    kind: Option<FontKind>,
    /// Show only the embroidery fonts for small text (P11 Milestone 2).
    small_text: bool,
    selected: Option<FontRef>,
    preview: Option<Preview>,
    form: Option<ConvertForm>,
    /// A conversion that runs in the background.
    job: Option<Receiver<Result<PathBuf, String>>>,
}

/// A file name from a font name: letters, digits, '-' and '_' only.
pub fn file_name(name: &str) -> String {
    let s: String = name
        .trim()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let s = s.trim_matches('-').to_string();
    if s.is_empty() { "font".into() } else { s }
}

impl AppState {
    /// Gives the selected lettering objects the font, as one undo step.
    /// Returns the number of objects that changed.
    pub fn use_font_for_selection(&mut self, font: &FontRef) -> usize {
        let ids = self.sel();
        let mut changed = Vec::new();
        for id in ids {
            let Some(ObjectKind::Lettering(l)) = self.doc.design().object(id).map(|o| o.kind.clone()) else {
                continue;
            };
            let mut n = *l;
            n.font = font.clone();
            if relayout(self, &mut n) {
                changed.push((id, n));
            }
        }
        if changed.is_empty() {
            return 0;
        }
        let count = changed.len();
        self.doc.edit("Change font", |d| {
            for (id, n) in changed {
                if let Some(o) = d.object_mut(id) {
                    o.kind = ObjectKind::Lettering(Box::new(n));
                }
            }
        });
        self.letter_font = font.clone();
        count
    }

    fn preview_of(&mut self, font: &FontRef) -> Option<Preview> {
        let any = self.fonts.get(font)?;
        let src = any.source();
        let l = Lettering::new(PREVIEW_TEXT, font.clone(), 10.0, Point::new(0.0, 0.0));
        let letters = emb_letter::layout(&l, src);
        let notes = match &any {
            emb_letter::AnyFont::Embroidery(e) => e.notes.clone(),
            emb_letter::AnyFont::TrueType(t) => t.copyright().unwrap_or_default(),
        };
        Some(Preview {
            font: font.clone(),
            letters,
            range: src.size_range(LetterStitch::Auto),
            notes,
        })
    }
}

/// The shapes of the letters, as screen rings (each part alone).
fn draw_letters(painter: &egui::Painter, rect: Rect, letters: &[Letter], color: Color32) {
    let Some(bb) = letters
        .iter()
        .flat_map(|l| l.parts.iter())
        .filter_map(ObjectKind::control_bbox)
        .reduce(BBox::union)
    else {
        return;
    };
    let pad = 8.0;
    let s = ((rect.width() - 2.0 * pad) as f64 / bb.width().max(1e-6))
        .min((rect.height() - 2.0 * pad) as f64 / bb.height().max(1e-6));
    let off = Vec2::new(
        (rect.width() - (bb.width() * s) as f32) / 2.0,
        (rect.height() - (bb.height() * s) as f32) / 2.0,
    );
    let to = |p: Point| rect.min + off + Vec2::new(((p.x - bb.min.x) * s) as f32, ((p.y - bb.min.y) * s) as f32);
    let ring = |path: &emb_geom::Path| -> Vec<Vec<Pos2>> {
        path.flatten(0.05)
            .into_iter()
            .map(|pl| pl.points.into_iter().map(to).collect())
            .collect()
    };
    for part in letters.iter().flat_map(|l| l.parts.iter()) {
        let rings: Vec<Vec<Pos2>> = match part {
            ObjectKind::Satin { geometry, .. } => {
                let (a, b) = (ring(&geometry.rail_a), ring(&geometry.rail_b));
                let closed = geometry.rail_a.flatten(0.05).first().is_some_and(|p| p.closed);
                if closed {
                    a.into_iter().chain(b).collect()
                } else {
                    let mut r: Vec<Pos2> = a.into_iter().flatten().collect();
                    r.extend(b.into_iter().flatten().rev());
                    vec![r]
                }
            }
            ObjectKind::Fill { outer, holes, .. } => {
                ring(outer).into_iter().chain(holes.iter().flat_map(&ring)).collect()
            }
            _ => Vec::new(),
        };
        painter.add(Shape::mesh(crate::applique::scanline_mesh(&rings, color, rect)));
    }
}

/// The Fonts window.
pub fn window(ctx: &egui::Context, st: &mut AppState) {
    // A finished conversion.
    if let Some(rx) = &st.fonts_window.job
        && let Ok(result) = rx.try_recv()
    {
        st.fonts_window.job = None;
        match result.and_then(|path| st.fonts.add_user_font(&path)) {
            Ok(r) => {
                st.message = crate::i18n::fill("Embroidery font {} is ready.", &[&(r.label())]);
                st.fonts_window.selected = Some(r.clone());
                st.fonts_window.preview = None;
                st.fonts_window.kind = None;
            }
            Err(e) => st.message = crate::i18n::fill("The font was not converted: {}", &[&e]),
        }
    }
    if st.fonts_window.job.is_some() {
        ctx.request_repaint_after(std::time::Duration::from_millis(200));
    }
    if !st.fonts_window.open {
        return;
    }
    st.fonts.start_scan();
    let mut open = true;
    crate::dock::docked(
        egui::Window::new(tr("Fonts")).id(egui::Id::new("Fonts")),
        ctx,
        egui::Id::new("Fonts"),
        st.canvas_rect,
    )
    .open(&mut open)
    .default_size([620.0, 420.0])
    .show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut st.fonts_window.filter)
                    .hint_text(tr("Search"))
                    .desired_width(180.0),
            );
            let w = &mut st.fonts_window;
            ui.selectable_value(&mut w.kind, None, tr("All"));
            ui.selectable_value(&mut w.kind, Some(FontKind::Embroidery), tr("Embroidery"));
            ui.selectable_value(&mut w.kind, Some(FontKind::TrueType), tr("TrueType"));
            ui.checkbox(&mut w.small_text, tr("For small text"))
                .on_hover_text(tr("Embroidery fonts for letters from 4 mm high."));
            if !st.fonts.scanned {
                ui.weak(tr("Finding the fonts of this computer…"));
            }
        });
        ui.separator();
        ui.columns(2, |cols| {
            list(&mut cols[0], st);
            details(&mut cols[1], st);
        });
    });
    st.fonts_window.open = open;
}

fn list(ui: &mut egui::Ui, st: &mut AppState) {
    let filter = st.fonts_window.filter.to_lowercase();
    let kind = st.fonts_window.kind;
    let entries: Vec<FontRef> = st
        .fonts
        .entries
        .iter()
        .map(|e| e.font.clone())
        .filter(|f| kind.is_none_or(|k| f.kind == k))
        .filter(|f| f.label().to_lowercase().contains(&filter))
        .collect();
    let entries: Vec<FontRef> = if st.fonts_window.small_text {
        entries.into_iter().filter(|f| is_small_text(st, f)).collect()
    } else {
        entries
    };
    egui::ScrollArea::vertical().id_salt("font_list").show(ui, |ui| {
        for f in entries {
            let on = st.fonts_window.selected.as_ref().is_some_and(|s| s.same(&f));
            let r = ui.selectable_label(on, format!("{}  ·  {}", f.label(), kind_name(f.kind)));
            if r.clicked() {
                st.fonts_window.selected = Some(f);
                st.fonts_window.form = None;
            }
        }
    });
}

/// True for an embroidery font for small text.
fn is_small_text(st: &mut AppState, f: &FontRef) -> bool {
    f.kind == FontKind::Embroidery
        && matches!(st.fonts.get(f), Some(emb_letter::AnyFont::Embroidery(e)) if e.small_text)
}

fn details(ui: &mut egui::Ui, st: &mut AppState) {
    let Some(sel) = st.fonts_window.selected.clone() else {
        ui.weak(tr("Select a font to see a preview."));
        return;
    };
    if st.fonts_window.preview.as_ref().is_none_or(|p| !p.font.same(&sel)) {
        st.fonts_window.preview = st.preview_of(&sel);
    }
    let Some(p) = st.fonts_window.preview.clone() else {
        ui.weak(tr("This font cannot be read."));
        return;
    };
    ui.strong(sel.label());
    ui.weak(match sel.kind {
        FontKind::Embroidery => tr("Embroidery font: the letters are ready-made embroidery objects."),
        FontKind::TrueType => tr("TrueType font: the letters are made from the outlines."),
    });
    if is_small_text(st, &sel) {
        ui.label(tr("For small text: letters from 4 mm high."));
    }
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 90.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 4.0, ui.visuals().extreme_bg_color);
    draw_letters(ui.painter(), rect, &p.letters, ui.visuals().text_color());
    if let Some((lo, hi)) = p.range {
        ui.label(crate::i18n::fill(
            "Size range: {}–{} mm (capital height)",
            &[&format!("{:.0}", lo), &format!("{:.0}", hi)],
        ));
    }
    if !p.notes.is_empty() {
        ui.add(egui::Label::new(egui::RichText::new(&p.notes).small().weak()).wrap());
    }
    ui.separator();
    let has_lettering = st.sel().into_iter().any(|id| {
        st.doc
            .design()
            .object(id)
            .is_some_and(|o| matches!(o.kind, ObjectKind::Lettering(_)))
    });
    if ui
        .add_enabled(has_lettering, egui::Button::new(tr("Use for the selected lettering")))
        .on_disabled_hover_text(tr("Select a lettering object first."))
        .clicked()
    {
        let n = st.use_font_for_selection(&sel);
        st.message = crate::i18n::fill(
            "{} uses {}.",
            &[&(if n == 1 { "1 object" } else { "The objects" }), &(sel.label())],
        );
    }
    if sel.kind == FontKind::TrueType {
        convert_ui(ui, st, &sel);
    }
}

fn convert_ui(ui: &mut egui::Ui, st: &mut AppState, sel: &FontRef) {
    if st.fonts_window.job.is_some() {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(tr("Converting…"));
        });
        return;
    }
    let Some(form) = st.fonts_window.form.as_mut().filter(|f| f.source.same(sel)) else {
        if ui
            .button(tr("Convert to embroidery font…"))
            .on_hover_text(tr("Make an embroidery font from this font, with a size range."))
            .clicked()
        {
            st.fonts_window.form = Some(ConvertForm {
                source: sel.clone(),
                name: sel.label(),
                stitch: LetterStitch::Auto,
            });
        }
        return;
    };
    ui.strong(tr("Convert to embroidery font"));
    ui.horizontal(|ui| {
        ui.label(tr("Name"));
        ui.text_edit_singleline(&mut form.name);
    });
    ui.horizontal(|ui| {
        ui.label(tr("Stitch type"));
        for (s, label) in [
            (LetterStitch::Auto, tr("Automatic")),
            (LetterStitch::Satin, tr("Satin")),
            (LetterStitch::Fill, tr("Fill")),
        ] {
            ui.selectable_value(&mut form.stitch, s, label);
        }
    });
    ui.add(
        egui::Label::new(
            egui::RichText::new(tr(
                "Check that the license of the font lets you change it. An OFL font with a Reserved \
                 Font Name needs a new name.",
            ))
            .small()
            .weak(),
        )
        .wrap(),
    );
    let form = form.clone();
    let Some(dir) = crate::lettering::user_fonts_dir() else {
        ui.weak(tr("There is no folder for user fonts."));
        return;
    };
    ui.horizontal(|ui| {
        if ui
            .add_enabled(!form.name.trim().is_empty(), egui::Button::new(tr("Convert")))
            .clicked()
        {
            start_convert(st, form, dir);
        }
        if ui.button(tr("Cancel")).clicked() {
            st.fonts_window.form = None;
        }
    });
}

fn start_convert(st: &mut AppState, form: ConvertForm, dir: PathBuf) {
    let Some(emb_letter::AnyFont::TrueType(font)) = st.fonts.get(&form.source) else {
        st.message = tr("The font cannot be read.").into();
        return;
    };
    let (tx, rx) = channel();
    st.fonts_window.job = Some(rx);
    st.fonts_window.form = None;
    std::thread::spawn(move || {
        let run = || -> Result<PathBuf, String> {
            std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
            let name = form.name.trim();
            let f = emb_letter::embfont::convert(&font, name, "", form.stitch, &emb_letter::embfont::default_chars());
            let path = dir.join(format!("{}.{}", file_name(name), emb_letter::embfont::FILE_EXTENSION));
            f.save(&path)?;
            Ok(path)
        };
        tx.send(run()).ok();
    });
}

#[cfg(test)]
mod tests {
    use emb_model::{HoopCatalog, MachineCatalog};

    use super::*;
    use crate::lettering::LetteringDialog;

    #[test]
    fn the_fonts_for_small_text_are_marked() {
        let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
        let entries: Vec<FontRef> = st.fonts.entries.iter().map(|e| e.font.clone()).collect();
        let small: Vec<String> = entries
            .iter()
            .filter(|f| is_small_text(&mut st, f))
            .map(|f| f.label())
            .collect();
        assert_eq!(small.len(), 4, "{small:?}");
        assert!(small.iter().any(|s| s.contains("Atkinson Hyperlegible")), "{small:?}");
        let bundled = entries.iter().filter(|f| f.kind == FontKind::Embroidery).count();
        assert!(bundled >= 30, "all bundled embroidery fonts are in the list: {bundled}");
    }

    #[test]
    fn file_names() {
        assert_eq!(file_name("Block Sans"), "block-sans");
        assert_eq!(file_name("  é/x "), "x");
        assert_eq!(file_name("***"), "font");
    }

    #[test]
    fn previews_and_font_change_of_the_selection() {
        let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
        let block = FontRef::embroidery("Block Sans", "");
        let p = st.preview_of(&block).unwrap();
        assert!(p.letters.len() >= 10);
        assert!(p.range.is_some());
        assert!(p.notes.contains("Lato"));
        assert!(st.preview_of(&FontRef::new("Pacifico", "Regular")).is_some());

        st.add_lettering(LetteringDialog::new(Point::new(0.0, 0.0), "Max", block.clone(), 20.0));
        let id = st.doc.design().objects()[0].id;
        st.selection.set(id);
        let anton = FontRef::embroidery("Anton", "");
        assert_eq!(st.use_font_for_selection(&anton), 1);
        let ObjectKind::Lettering(l) = &st.doc.design().objects()[0].kind else {
            panic!()
        };
        assert!(l.font.same(&anton));
        assert!(l.range_mm.is_some());
        st.doc.undo();
        let ObjectKind::Lettering(l) = &st.doc.design().objects()[0].kind else {
            panic!()
        };
        assert!(l.font.same(&block));
        let mut small = (**l).clone();

        // Below the size range of the font: a warning in the design checks.
        // No warning of the object (the density map can list the joins of
        // the satin letters; ADR 0044).
        assert!(st.design_info().issues.iter().all(|i| i.object.is_none()));
        small.height_mm = 4.0;
        assert!(relayout(&mut st, &mut small));
        st.doc.edit("Smaller", |d| {
            d.object_mut(id).unwrap().kind = ObjectKind::Lettering(Box::new(small))
        });
        let issues: Vec<String> = st.design_info().issues.iter().map(|i| i.text.clone()).collect();
        assert!(
            issues
                .iter()
                .any(|t| t.contains("smaller than the size range of Block Sans")),
            "{issues:?}"
        );
    }

    #[test]
    fn user_fonts_from_a_folder() {
        let dir = std::env::temp_dir().join(format!("emb-user-fonts-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let font = emb_letter::bundled::font("Pacifico", "Regular").unwrap();
        let f = emb_letter::embfont::convert(&font, "My Script", "", LetterStitch::Satin, &['A', 'b']);
        f.save(&dir.join("my-script.embfont")).unwrap();
        std::fs::write(dir.join("broken.embfont"), "{}").unwrap();
        let mut lib = crate::lettering::FontLibrary::default();
        lib.add_user_fonts(&dir);
        let r = FontRef::embroidery("My Script", "");
        assert!(lib.entries.iter().any(|e| e.font.same(&r)));
        let any = lib.get(&r).unwrap();
        assert!(any.source().letter('A').is_some() && any.source().letter('z').is_none());
        std::fs::remove_dir_all(dir).ok();
    }
}
