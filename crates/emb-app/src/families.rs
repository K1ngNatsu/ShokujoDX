//! The Families window (P17 Milestone 8, ADR 0118): make a motif, a
//! program-split pattern, a carving stamp or a clipart item from a family
//! and its parameters (`emb_model::family`), see it, and use it for the
//! selection or keep it in the library.
//!
//! The Motifs window, the pattern and stamp rows of Object Properties and
//! the Clipart tab open it for their kind.

use std::path::PathBuf;

use eframe::egui::{self, Color32, Pos2, Sense, Stroke, Vec2};
use emb_geom::{Point, Transform};
use emb_model::family::{self, Family, Item, Kind};
use emb_model::{ColumnPattern, DesignObject, FillPattern, ObjectKind, project};

use crate::app::AppState;
use crate::i18n::tr;

#[derive(Debug, Clone)]
pub struct FamiliesWindow {
    pub open: bool,
    pub kind: Kind,
    /// The index of the family in the families of the kind.
    pub family: usize,
    /// The values of the parameters of the family.
    pub values: Vec<f64>,
    /// The name of the item to keep (empty: its description).
    pub name: String,
}

impl Default for FamiliesWindow {
    fn default() -> Self {
        let mut w = FamiliesWindow {
            open: false,
            kind: Kind::Motif,
            family: 0,
            values: Vec::new(),
            name: String::new(),
        };
        w.choose(0);
        w
    }
}

impl FamiliesWindow {
    /// Opens the window for a kind (the first family, if the kind changes).
    pub fn show(&mut self, kind: Kind) {
        self.open = true;
        if self.kind != kind {
            self.kind = kind;
            self.choose(0);
        }
    }

    /// The family that the window shows.
    pub fn family(&self) -> &'static Family {
        let all = family::families_of(self.kind);
        all[self.family.min(all.len() - 1)]
    }

    /// Takes the family `index` of the kind, with its default values.
    pub fn choose(&mut self, index: usize) {
        self.family = index;
        self.values = self.family().defaults();
        self.name.clear();
    }

    /// The description of the item (`Waves: 2, 0.3`).
    pub fn description(&self) -> String {
        self.family().description(&self.values)
    }

    /// The item of the window. Its name is the typed name, or its
    /// description.
    pub fn item(&self) -> Item {
        let name = self.name.trim();
        let name = if name.is_empty() {
            self.description()
        } else {
            name.to_string()
        };
        self.family().item(&name, &self.values)
    }
}

impl AppState {
    /// Uses the item of the Families window for the selection: a motif as
    /// the Motifs window does; a split pattern for the selected fills and
    /// program-split columns (a tatami fill becomes a program-split fill);
    /// a stamp in the middle of each selected fill that takes stamps. A
    /// clipart item goes to the middle of the view. Returns the number of
    /// objects that changed (or that came).
    pub fn use_family_item(&mut self) -> usize {
        let ids = self.sel();
        match self.families_window.item() {
            Item::Motif(m) => self.apply_motif(&ids, &m),
            Item::Split(p) => {
                let d = self.doc.design();
                let takes = |k: &ObjectKind| match k {
                    ObjectKind::Fill { params, .. } => matches!(
                        params.pattern,
                        FillPattern::Tatami | FillPattern::ProgramSplit { .. } | FillPattern::FlexiSplit { .. }
                    ),
                    ObjectKind::Satin { params, .. } => matches!(params.pattern, ColumnPattern::ProgramSplit { .. }),
                    _ => false,
                };
                let ids: Vec<_> = ids
                    .into_iter()
                    .filter(|id| d.object(*id).is_some_and(|o| takes(&o.kind)))
                    .collect();
                if ids.is_empty() {
                    return 0;
                }
                let n = ids.len();
                self.doc.edit(crate::i18n::fill("Pattern {}", &[&p.name]), |d| {
                    for id in ids {
                        match d.object_mut(id).map(|o| &mut o.kind) {
                            Some(ObjectKind::Fill { params, .. }) => match &mut params.pattern {
                                FillPattern::ProgramSplit { pattern, .. } | FillPattern::FlexiSplit { pattern, .. } => {
                                    *pattern = p.clone()
                                }
                                other => {
                                    *other = FillPattern::ProgramSplit {
                                        pattern: p.clone(),
                                        scale: 1.0,
                                    }
                                }
                            },
                            Some(ObjectKind::Satin { params, .. }) => params.split_pattern = Some(p.clone()),
                            _ => {}
                        }
                    }
                });
                n
            }
            Item::Stamp(stamp) => {
                let d = self.doc.design();
                let places: Vec<_> = ids
                    .into_iter()
                    .filter_map(|id| {
                        let o = d.object(id)?;
                        let ObjectKind::Fill { params, .. } = &o.kind else {
                            return None;
                        };
                        let b = o.control_bbox().filter(|_| params.pattern.takes_stamps())?;
                        Some((id, b.center(), (b.width().min(b.height()) * 0.6).max(5.0)))
                    })
                    .collect();
                if places.is_empty() {
                    return 0;
                }
                let n = places.len();
                self.doc.edit(crate::i18n::fill("Stamp {}", &[&stamp.name]), |d| {
                    for (id, center, size_mm) in places {
                        if let Some(ObjectKind::Fill { params, .. }) = d.object_mut(id).map(|o| &mut o.kind) {
                            params.stamps.push(emb_model::StampPlacement {
                                stamp: stamp.clone(),
                                center,
                                size_mm,
                                angle_deg: 0.0,
                            });
                        }
                    }
                });
                n
            }
            Item::Clipart(item) => {
                let objects: Vec<DesignObject> = item.objects().iter().map(|o| (**o).clone()).collect();
                let inv = self
                    .doc
                    .design()
                    .rotation_transform()
                    .inverse()
                    .unwrap_or(Transform::IDENTITY);
                let target = inv.apply(self.view.center);
                let new = self.doc.edit(tr("Insert clipart"), |d| {
                    let new = d.paste_objects(&objects, &item.threads);
                    d.center_objects_at(&new, target);
                    new
                });
                let n = new.len();
                self.selection.set_all(new);
                n
            }
        }
    }

    /// Keeps the item of the Families window in the library of the user: a
    /// motif in the motifs folder, a clipart item in the clipart folder.
    /// Returns its file. A pattern or a stamp has no library: the object
    /// that uses it keeps it.
    pub fn keep_family_item(&mut self) -> Result<PathBuf, String> {
        match self.families_window.item() {
            Item::Motif(m) => {
                if crate::motifs::library()
                    .iter()
                    .any(|(x, _)| x.name.eq_ignore_ascii_case(&m.name))
                {
                    return Err(crate::i18n::fill(
                        "A motif named {} is already in the library.",
                        &[&m.name],
                    ));
                }
                let dir = crate::motifs::motifs_dir().ok_or(tr("No folder for motifs."))?;
                std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
                let path = dir.join(crate::motifs::file_name(&m.name));
                std::fs::write(&path, serde_json::to_string_pretty(&m).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
                Ok(path)
            }
            Item::Clipart(d) => {
                let dir = self.clipart.dir.clone().ok_or(tr("There is no clipart folder."))?;
                let plain = d.name.replace(':', "");
                let name = crate::styles::valid_name(&plain).ok_or(tr("Type a name without / \\ : * ? \" < > |"))?;
                let path = dir.join(format!("{name}.{}", project::FILE_EXTENSION));
                if path.exists() {
                    return Err(crate::i18n::fill(
                        "A clipart item \"{}\" is there already. Use another name.",
                        &[&name],
                    ));
                }
                std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
                project::save(&d, &path).map_err(|e| e.to_string())?;
                self.scan_clipart();
                Ok(path)
            }
            Item::Split(_) | Item::Stamp(_) => Err(tr("The object that uses a pattern or a stamp keeps it.").into()),
        }
    }
}

/// Draws the item in `size`.
fn preview(ui: &mut egui::Ui, item: &Item, size: Vec2) {
    let ink = ui.visuals().text_color();
    if let Item::Motif(m) = item {
        crate::motifs::preview(ui, m, size, ink);
        return;
    }
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let painter = ui.painter().with_clip_rect(rect);
    painter.rect_filled(rect, 4.0, ui.visuals().extreme_bg_color);
    let inner = rect.shrink(8.0);
    match item {
        Item::Motif(_) => {}
        Item::Split(p) => {
            // The needle points of the rows, at 0.4 mm between two rows.
            let mm = 7.0_f32;
            let (tile, step) = (p.width_mm as f32 * mm, 0.4 * mm);
            let mut row = 0;
            while (row as f32) * step < inner.height() {
                let y = inner.top() + row as f32 * step;
                let points = &p.rows[row % p.rows.len()];
                let mut k = 0.0_f32;
                while k * tile < inner.width() {
                    for f in points {
                        let x = inner.left() + (k + *f as f32) * tile;
                        if x <= inner.right() {
                            painter.circle_filled(Pos2::new(x, y), 1.2, ink);
                        }
                    }
                    k += 1.0;
                }
                row += 1;
            }
        }
        Item::Stamp(s) => {
            let side = inner.width().min(inner.height());
            let at = |q: &Point| inner.center() + Vec2::new(q.x as f32 * side, q.y as f32 * side);
            for line in &s.lines {
                painter.add(egui::Shape::line(line.iter().map(at).collect(), Stroke::new(1.5, ink)));
            }
        }
        Item::Clipart(d) => {
            let Some(b) = d
                .objects()
                .iter()
                .filter_map(|o| o.kind.control_bbox())
                .reduce(emb_geom::BBox::union)
            else {
                return;
            };
            let scale =
                (f64::from(inner.width()) / b.width().max(1.0)).min(f64::from(inner.height()) / b.height().max(1.0));
            let c = b.center();
            let at = |q: &Point| inner.center() + Vec2::new(((q.x - c.x) * scale) as f32, ((q.y - c.y) * scale) as f32);
            // The outlines of the fills and the lines, in the thread colors.
            for o in d.objects() {
                let color = d
                    .thread(o.thread)
                    .map_or(ink, |t| Color32::from_rgb(t.color.0, t.color.1, t.color.2));
                let mut paths = Vec::new();
                match &o.kind {
                    ObjectKind::Fill { outer, holes, .. } => {
                        paths.push((outer, 2.5));
                        paths.extend(holes.iter().map(|h| (h, 2.5)));
                    }
                    ObjectKind::Running { path, .. } => paths.push((path, 1.5)),
                    _ => {}
                }
                for (path, width) in paths {
                    for line in path.flatten(0.1) {
                        let mut pts: Vec<Pos2> = line.points.iter().map(at).collect();
                        if line.closed
                            && let Some(first) = pts.first().copied()
                        {
                            pts.push(first);
                        }
                        painter.add(egui::Shape::line(pts, Stroke::new(width, color)));
                    }
                }
            }
        }
    }
}

/// The Families window.
pub fn window(ctx: &egui::Context, st: &mut AppState) {
    if !st.families_window.open {
        return;
    }
    let mut w = st.families_window.clone();
    let mut open = true;
    let (mut use_it, mut keep) = (false, false);
    let has_selection = !st.selection.is_empty();
    crate::dock::docked(
        egui::Window::new(tr("Families")).id(egui::Id::new("Families")),
        ctx,
        egui::Id::new("Families"),
        st.canvas_rect,
    )
    .open(&mut open)
    .collapsible(false)
    .resizable(false)
    .default_width(380.0)
    .show(ctx, |ui| {
        ui.label(tr(
            "A family makes an item from some numbers. Change the numbers to make an item of your own.",
        ));
        ui.horizontal(|ui| {
            for kind in Kind::ALL {
                if ui.selectable_label(w.kind == kind, tr(kind.name())).clicked() && w.kind != kind {
                    w.kind = kind;
                    w.choose(0);
                }
            }
        });
        let all = family::families_of(w.kind);
        let f = w.family();
        ui.horizontal(|ui| {
            ui.label(tr("Family"));
            let mut pick = None;
            egui::ComboBox::from_id_salt("family")
                .selected_text(f.name)
                .height(320.0)
                .show_ui(ui, |ui| {
                    for (i, g) in all.iter().enumerate() {
                        if ui.selectable_label(i == w.family, g.name).clicked() {
                            pick = Some(i);
                        }
                    }
                });
            if let Some(i) = pick {
                w.choose(i);
            }
        });
        let f = w.family();
        ui.horizontal_wrapped(|ui| {
            ui.label(tr("Built-in items"));
            for (name, values) in f.preset_values() {
                if ui
                    .selectable_label(w.values == values, name)
                    .on_hover_text(f.description(&values))
                    .clicked()
                {
                    w.values = values;
                }
            }
        });
        w.values = f.values(&w.values);
        egui::Grid::new("family_params").num_columns(2).show(ui, |ui| {
            for (p, v) in f.params.iter().zip(w.values.iter_mut()) {
                ui.label(tr(p.name));
                if p.whole && p.min == 0.0 && p.max == 1.0 {
                    let mut on = *v >= 0.5;
                    ui.checkbox(&mut on, "");
                    *v = f64::from(u8::from(on));
                } else {
                    let slider = egui::Slider::new(v, p.min..=p.max).clamping(egui::SliderClamping::Always);
                    ui.add(if p.whole {
                        slider.integer()
                    } else {
                        slider.fixed_decimals(2)
                    });
                }
                ui.end_row();
            }
        });
        let item = w.item();
        preview(ui, &item, Vec2::new(ui.available_width().max(340.0), 130.0));
        ui.horizontal(|ui| {
            ui.label(tr("Description"));
            ui.add(egui::Label::new(egui::RichText::new(w.description()).monospace()).selectable(true))
                .on_hover_text(tr(
                    "The name of the family and its numbers. The same description gives the same item.",
                ));
        });
        ui.separator();
        let (use_text, use_tip) = match w.kind {
            Kind::Motif => (
                tr("Use for the selection"),
                tr("Lines become motif runs, satin columns motif columns, fills motif fills."),
            ),
            Kind::Split => (
                tr("Use for the selection"),
                tr("The selected fills get the pattern. A tatami fill becomes a program-split fill."),
            ),
            Kind::Stamp => (
                tr("Add to the selected fills"),
                tr("The stamp goes in the middle of each selected fill. Move it and size it in Object Properties."),
            ),
            Kind::Clipart => (tr("Insert"), tr("Put the item at the center of the view.")),
        };
        ui.horizontal(|ui| {
            let ready = has_selection || w.kind == Kind::Clipart;
            use_it = ui
                .add_enabled(ready, egui::Button::new(use_text))
                .on_hover_text(use_tip)
                .on_disabled_hover_text(tr("Select objects first."))
                .clicked();
        });
        if matches!(w.kind, Kind::Motif | Kind::Clipart) {
            ui.horizontal(|ui| {
                ui.label(tr("Name"));
                let hint = w.description();
                ui.add(
                    egui::TextEdit::singleline(&mut w.name)
                        .hint_text(hint)
                        .desired_width(160.0),
                );
                keep = ui
                    .button(tr("Keep in the library"))
                    .on_hover_text(tr(
                        "A motif goes to your motifs, a clipart item to your clipart. Then it is in the lists.",
                    ))
                    .clicked();
            });
        }
    });
    w.open = open;
    st.families_window = w;
    if use_it {
        let name = st.families_window.item().name().to_string();
        let n = st.use_family_item();
        st.message = if n > 0 {
            crate::i18n::fill("{} used for {} objects.", &[&name, &n])
        } else {
            tr("No selected object can take this item.").to_string()
        };
    }
    if keep {
        let name = st.families_window.item().name().to_string();
        st.message = match st.keep_family_item() {
            Ok(_) => crate::i18n::fill("{} is in the library.", &[&name]),
            Err(e) => e,
        };
    }
}

#[cfg(test)]
mod tests {
    use emb_geom::{BBox, Path};
    use emb_model::{FillParams, RunningParams};

    use super::*;
    use crate::test_util::Harness;

    fn fill(h: &mut Harness) -> emb_model::ObjectId {
        h.st.doc.edit("Add", |d| {
            let t = d.add_thread("Red", emb_model::Rgb(200, 0, 0));
            d.add_object(
                t,
                ObjectKind::Fill {
                    outer: Path::rect(BBox::new(Point::new(0.0, 0.0), Point::new(40.0, 30.0))),
                    holes: Vec::new(),
                    params: FillParams::default(),
                },
            )
        })
    }

    #[test]
    fn the_window_makes_an_item_of_each_kind_and_draws_it() {
        let mut h = Harness::empty();
        for kind in Kind::ALL {
            h.st.families_window.show(kind);
            let n = family::families_of(kind).len();
            for i in 0..n {
                h.st.families_window.choose(i);
                let w = &h.st.families_window;
                assert_eq!(w.item().name(), w.description());
                // The description of the window gives the item again.
                assert_eq!(family::from_description(kind, &w.description()), Some(w.item()));
                h.frame(Vec::new());
            }
        }
        // A typed name is the name of the item.
        h.st.families_window.name = " Mine ".into();
        assert_eq!(h.st.families_window.item().name(), "Mine");
        // Another kind starts at its first family.
        h.st.families_window.choose(3);
        h.st.families_window.show(Kind::Motif);
        assert_eq!(
            (h.st.families_window.family, h.st.families_window.kind),
            (0, Kind::Motif)
        );
    }

    #[test]
    fn a_motif_a_pattern_and_a_stamp_for_the_selection() {
        let mut h = Harness::empty();
        let id = fill(&mut h);
        let line = h.st.doc.edit("Line", |d| {
            let t = d.threads[0].id;
            d.add_object(
                t,
                ObjectKind::Running {
                    path: Path::new()
                        .move_to(Point::new(0.0, 40.0))
                        .line_to(Point::new(60.0, 40.0)),
                    params: RunningParams::default(),
                },
            )
        });
        // Nothing selected: nothing changes.
        h.st.selection.clear();
        h.st.families_window.show(Kind::Split);
        assert_eq!(h.st.use_family_item(), 0);
        // A split pattern: the tatami fill becomes a program-split fill;
        // the line does not take it.
        h.st.selection.set_all(vec![id, line]);
        assert_eq!(h.st.use_family_item(), 1);
        let pattern = |h: &Harness| match &h.st.doc.design().object(id).unwrap().kind {
            ObjectKind::Fill { params, .. } => params.pattern.clone(),
            _ => panic!(),
        };
        let FillPattern::ProgramSplit { pattern: first, .. } = pattern(&h) else {
            panic!()
        };
        assert_eq!(first.name, h.st.families_window.description());
        // Another value: the pattern of the fill changes, in one undo step.
        h.st.families_window.values[0] = 8.0;
        assert_eq!(h.st.use_family_item(), 1);
        let FillPattern::ProgramSplit { pattern: second, .. } = pattern(&h) else {
            panic!()
        };
        assert!(second.width_mm == 8.0 && second != first);
        h.st.doc.undo();
        assert_eq!(
            pattern(&h),
            FillPattern::ProgramSplit {
                pattern: first,
                scale: 1.0
            }
        );
        // A stamp: in the middle of the fill.
        h.st.families_window.show(Kind::Stamp);
        assert_eq!(h.st.use_family_item(), 1);
        let ObjectKind::Fill { params, .. } = &h.st.doc.design().object(id).unwrap().kind else {
            panic!()
        };
        assert_eq!(params.stamps.len(), 1);
        assert!(params.stamps[0].center.distance(Point::new(20.0, 15.0)) < 1e-9);
        assert!((params.stamps[0].size_mm - 18.0).abs() < 1e-9);
        // A motif: the line becomes a motif run.
        h.st.families_window.show(Kind::Motif);
        assert_eq!(h.st.use_family_item(), 2);
        let ObjectKind::Running { params, .. } = &h.st.doc.design().object(line).unwrap().kind else {
            panic!()
        };
        assert_eq!(
            params.motif.as_ref().unwrap().motif.name,
            h.st.families_window.description()
        );
        assert!(h.st.plan_view().plan.stitches.stitch_count() > 100);
    }

    #[test]
    fn a_clipart_item_goes_to_the_design_and_to_the_library() {
        let mut h = Harness::empty();
        let dir = std::env::temp_dir().join(format!("emb-family-clipart-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        h.st.start_clipart(Some(dir.clone()));
        let before = h.st.clipart.index.entries.len();
        h.st.families_window.show(Kind::Clipart);
        let Item::Clipart(item) = h.st.families_window.item() else {
            panic!()
        };
        assert_eq!(h.st.use_family_item(), item.objects().len());
        assert_eq!(h.st.doc.design().objects().len(), item.objects().len());
        assert_eq!(h.st.sel().len(), item.objects().len());
        // Keep it: a file in the clipart folder, found by the tab. The
        // name of the file has no ":".
        let path = h.st.keep_family_item().unwrap();
        assert!(path.is_file() && !path.file_name().unwrap().to_string_lossy().contains(':'));
        assert_eq!(h.st.clipart.index.entries.len(), before + 1);
        // The same item again: refused.
        assert!(h.st.keep_family_item().is_err());
        // A pattern has no library.
        h.st.families_window.show(Kind::Split);
        assert!(h.st.keep_family_item().is_err());
        std::fs::remove_dir_all(dir).ok();
    }
}
