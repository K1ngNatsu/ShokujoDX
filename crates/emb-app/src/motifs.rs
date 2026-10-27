//! The Motifs window (P5 Milestone 4): the motif library (built-in motifs
//! and the user's `.embmotif` files in the `motifs` folder next to the
//! settings file), use a motif for the selection, create a motif from the
//! selected objects, rename and delete the user's motifs.

use std::path::PathBuf;

use eframe::egui::{self, Color32, Sense, Stroke, Vec2};
use emb_geom::Point;
use emb_model::motif::FILE_EXTENSION;
use emb_model::{ColumnPattern, FillPattern, Motif, MotifRun, ObjectId, ObjectKind, SatinParams};

use crate::app::AppState;
use crate::i18n::tr;

#[derive(Debug, Clone, Default)]
pub struct MotifsWindow {
    pub open: bool,
    /// The index of the motif in the library.
    pub selected: usize,
    /// The name of a new motif, or the new name of the selected one.
    pub name: String,
}

/// The folder of the user's motifs.
pub fn motifs_dir() -> Option<PathBuf> {
    crate::workspace::settings_path()?.parent().map(|d| d.join("motifs"))
}

/// The built-in motifs and the user's (with their files).
pub fn library() -> Vec<(Motif, Option<PathBuf>)> {
    let mut all: Vec<(Motif, Option<PathBuf>)> = emb_model::motif::builtin().into_iter().map(|m| (m, None)).collect();
    if let Some(Ok(rd)) = motifs_dir().map(std::fs::read_dir) {
        let mut user: Vec<(Motif, Option<PathBuf>)> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some(FILE_EXTENSION))
            .filter_map(|p| {
                let m: Motif = serde_json::from_str(&std::fs::read_to_string(&p).ok()?).ok()?;
                Some((m, Some(p)))
            })
            .collect();
        user.sort_by_key(|(m, _)| m.name.to_lowercase());
        all.extend(user);
    }
    all
}

/// A file name from a motif name.
fn file_name(name: &str) -> String {
    let stem: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || " -_".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!("{stem}.{FILE_EXTENSION}")
}

/// The motif of an object, if it uses one.
pub fn motif_of(k: &ObjectKind) -> Option<&Motif> {
    match k {
        ObjectKind::Running { params, .. } => params.motif.as_ref().map(|m| &m.motif),
        ObjectKind::Satin { params, .. } if matches!(params.pattern, ColumnPattern::Motif { .. }) => {
            params.motif.as_ref()
        }
        ObjectKind::Fill { params, .. } => match &params.pattern {
            FillPattern::Motif { motif, .. } => Some(motif),
            _ => None,
        },
        _ => None,
    }
}

impl AppState {
    /// Uses `m` for the objects `ids`: lines become motif runs, satin
    /// columns motif columns, fills motif fills. Sizes that the objects
    /// have stay. One undo step; returns the number of objects changed.
    pub fn apply_motif(&mut self, ids: &[ObjectId], m: &Motif) -> usize {
        let d = self.doc.design();
        let targets: Vec<ObjectId> = ids
            .iter()
            .copied()
            .filter(|id| {
                matches!(
                    d.object(*id).map(|o| &o.kind),
                    Some(ObjectKind::Running { .. } | ObjectKind::Satin { .. } | ObjectKind::Fill { .. })
                )
            })
            .collect();
        if targets.is_empty() {
            self.message = tr("Select lines, satin columns or fills to use the motif.").into();
            return 0;
        }
        self.doc.edit(format!("Motif {}", m.name), |d| {
            for id in &targets {
                let Some(o) = d.object_mut(*id) else { continue };
                match &mut o.kind {
                    ObjectKind::Running { params, .. } => {
                        let mut r = params.motif.take().unwrap_or_else(|| MotifRun::new(m.clone()));
                        r.motif = m.clone();
                        params.motif = Some(r);
                        params.ripple = None;
                    }
                    ObjectKind::Satin { params, .. } => {
                        if !matches!(params.pattern, ColumnPattern::Motif { .. }) {
                            *params = SatinParams {
                                pull_compensation_mm: params.pull_compensation_mm,
                                ..SatinParams::with_pattern(ColumnPattern::Motif { aspect: 1.0 })
                            };
                        }
                        params.motif = Some(m.clone());
                    }
                    ObjectKind::Fill { params, .. } => match &mut params.pattern {
                        FillPattern::Motif { motif, .. } => *motif = m.clone(),
                        _ => {
                            let base = params.clone();
                            *params = base.with_pattern(FillPattern::Motif {
                                motif: m.clone(),
                                length_mm: 6.0,
                                height_mm: 4.0,
                                stagger: 0.5,
                                gradient: 1.0,
                                globe: false,
                            });
                        }
                    },
                    _ => {}
                }
            }
        });
        targets.len()
    }

    /// The needle points of the selected objects in sew order.
    fn selection_points(&self) -> Vec<Point> {
        let d = self.doc.design();
        let mut needle = Point::ZERO;
        let mut pts = Vec::new();
        for id in self.sel() {
            let Some(o) = d.object(id) else { continue };
            let entry = emb_stitch::object::entry_for(o, needle).unwrap_or(needle);
            if let Ok(s) = emb_stitch::stitch_object(o, entry) {
                for r in s.runs {
                    pts.extend(r.points);
                }
                if let Some(&last) = pts.last() {
                    needle = last;
                }
            }
        }
        pts
    }

    /// Saves the stitches of the selected objects as a new motif named
    /// `name` in the user's motifs folder. Returns its file.
    pub fn create_motif(&mut self, name: &str) -> Result<PathBuf, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Type a name for the motif.".into());
        }
        if library().iter().any(|(m, _)| m.name.eq_ignore_ascii_case(name)) {
            return Err(format!("A motif named {name} is already in the library."));
        }
        let m = Motif::from_points(name, &self.selection_points()).ok_or("Select the objects of the motif first.")?;
        let dir = motifs_dir().ok_or("No folder for motifs.")?;
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let path = dir.join(file_name(name));
        std::fs::write(&path, serde_json::to_string_pretty(&m).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        Ok(path)
    }
}

/// Renames the motif in `path` (the file follows the name).
fn rename(path: &std::path::Path, name: &str) -> Result<PathBuf, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Type the new name.".into());
    }
    let mut m: Motif =
        serde_json::from_str(&std::fs::read_to_string(path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    m.name = name.to_string();
    let new = path.with_file_name(file_name(name));
    std::fs::write(&new, serde_json::to_string_pretty(&m).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    if new != path {
        std::fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    Ok(new)
}

/// Draws the motif repeated three times in `size`.
pub fn preview(ui: &mut egui::Ui, m: &Motif, size: Vec2, color: Color32) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let h = m.height();
    let (lo, hi) = m.points.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| {
        (lo.min(p.y), hi.max(p.y))
    });
    let repeat = (rect.width() as f64 - 8.0) / 3.0;
    let scale = (repeat).min((rect.height() as f64 - 8.0) / h);
    let mid = (lo + hi) / 2.0;
    let y0 = rect.center().y as f64;
    let pts: Vec<egui::Pos2> = (0..3)
        .flat_map(|k| {
            m.points.iter().map(move |p| {
                egui::pos2(
                    (rect.left() as f64 + 4.0 + (f64::from(k) + p.x) * repeat) as f32,
                    (y0 + (p.y - mid) * scale) as f32,
                )
            })
        })
        .collect();
    ui.painter().add(egui::Shape::line(pts, Stroke::new(1.5, color)));
    resp
}

/// The Motifs window.
pub fn window(ctx: &egui::Context, st: &mut AppState) {
    if !st.motifs_window.open {
        return;
    }
    let all = library();
    let mut w = st.motifs_window.clone();
    w.selected = w.selected.min(all.len().saturating_sub(1));
    let mut open = true;
    let (mut use_it, mut create, mut do_rename, mut delete) = (false, false, false, false);
    let has_selection = !st.selection.is_empty();
    // The motif that the selected object uses, if it has one.
    let in_use: Option<String> = st
        .selection
        .single()
        .and_then(|id| st.doc.design().object(id))
        .and_then(|o| motif_of(&o.kind))
        .map(|m| m.name.clone());
    crate::dock::docked(
        egui::Window::new(tr("Motifs")).id(egui::Id::new("Motifs")),
        ctx,
        egui::Id::new("Motifs"),
        st.canvas_rect,
    )
    .open(&mut open)
    .collapsible(false)
    .resizable(false)
    .default_width(420.0)
    .show(ctx, |ui| {
        if let Some(name) = &in_use {
            ui.label(crate::i18n::fill("The selected object uses: {}", &[&name]));
        }
        let accent = ui.visuals().selection.bg_fill;
        let ink = ui.visuals().text_color();
        egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
            egui::Grid::new("motif_grid").spacing([6.0, 6.0]).show(ui, |ui| {
                for (i, (m, path)) in all.iter().enumerate() {
                    ui.vertical(|ui| {
                        let color = if i == w.selected { accent } else { ink };
                        let r = preview(ui, m, Vec2::new(90.0, 36.0), color);
                        let label = if path.is_some() {
                            format!("{} (yours)", m.name)
                        } else {
                            m.name.clone()
                        };
                        let l = ui.selectable_label(i == w.selected, label);
                        if r.clicked() || l.clicked() {
                            w.selected = i;
                            w.name = m.name.clone();
                        }
                        if r.double_clicked() || l.double_clicked() {
                            use_it = true;
                        }
                    });
                    if i % 4 == 3 {
                        ui.end_row();
                    }
                }
            });
        });
        ui.separator();
        ui.horizontal(|ui| {
            use_it |= ui
                .add_enabled(has_selection, egui::Button::new(tr("Use for the selection")))
                .on_hover_text(tr(
                    "Lines become motif runs, satin columns motif columns, fills motif fills.",
                ))
                .clicked();
        });
        ui.horizontal(|ui| {
            ui.label(tr("Name"));
            ui.add(egui::TextEdit::singleline(&mut w.name).desired_width(160.0));
            create |= ui
                .add_enabled(has_selection, egui::Button::new(tr("Create from selection")))
                .on_hover_text(tr(
                    "The stitches of the selected objects become a motif. The left and right middle of \
                         their box are its reference points.",
                ))
                .clicked();
        });
        let user = all.get(w.selected).and_then(|(_, p)| p.clone());
        ui.horizontal(|ui| {
            do_rename |= ui
                .add_enabled(user.is_some(), egui::Button::new(tr("Rename")))
                .clicked();
            delete |= ui
                .add_enabled(user.is_some(), egui::Button::new(tr("Delete")))
                .on_hover_text(tr(
                    "Delete the selected motif of yours from the library. Designs keep their copy.",
                ))
                .clicked();
        });
    });
    let selected = all.get(w.selected).cloned();
    st.motifs_window = MotifsWindow { open, ..w.clone() };
    if use_it && let Some((m, _)) = &selected {
        let ids = st.sel();
        let n = st.apply_motif(&ids, m);
        if n > 0 {
            st.message = crate::i18n::fill("{} used for {} objects.", &[&m.name, &n]);
        }
    }
    if create {
        match st.create_motif(&w.name) {
            Ok(_) => {
                st.message = crate::i18n::fill("Motif {} added to the library.", &[&(w.name.trim())]);
                if let Some(i) = library().iter().position(|(m, _)| m.name == w.name.trim()) {
                    st.motifs_window.selected = i;
                }
            }
            Err(e) => st.message = e,
        }
    }
    if let Some((_, Some(path))) = &selected {
        if do_rename {
            st.message = match rename(path, &w.name) {
                Ok(_) => format!("Renamed to {}.", w.name.trim()),
                Err(e) => e,
            };
        }
        if delete {
            st.message = match std::fs::remove_file(path) {
                Ok(()) => "Motif deleted.".into(),
                Err(e) => e.to_string(),
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use emb_geom::Path;
    use emb_model::{FillParams, RunningParams, SatinGeometry};

    use super::*;
    use crate::test_util::Harness;

    #[test]
    fn a_motif_for_a_line_a_column_and_a_fill_in_one_step() {
        let mut h = Harness::empty();
        let ids = h.st.doc.edit("Add", |d| {
            let t = d.add_thread("Red", emb_model::Rgb(200, 0, 0));
            [
                d.add_object(
                    t,
                    ObjectKind::Running {
                        path: Path::new().move_to(Point::ZERO).line_to(Point::new(30.0, 0.0)),
                        params: RunningParams::default(),
                    },
                ),
                d.add_object(
                    t,
                    ObjectKind::Satin {
                        geometry: SatinGeometry::from_centerline(
                            &Path::new()
                                .move_to(Point::new(0.0, 10.0))
                                .line_to(Point::new(30.0, 10.0)),
                            5.0,
                        ),
                        params: SatinParams::default(),
                    },
                ),
                d.add_object(
                    t,
                    ObjectKind::Fill {
                        outer: Path::rect(emb_geom::BBox::new(Point::new(0.0, 20.0), Point::new(30.0, 40.0))),
                        holes: Vec::new(),
                        params: FillParams::default(),
                    },
                ),
            ]
        });
        let heart = Motif::builtin("Heart").unwrap();
        assert_eq!(h.st.apply_motif(&ids, &heart), 3);
        assert_eq!(h.st.doc.undo_len(), 2);
        let d = h.st.doc.design();
        for id in ids {
            let o = d.object(id).unwrap();
            assert_eq!(
                motif_of(&o.kind).map(|m| m.name.as_str()),
                Some("Heart"),
                "{}",
                o.type_name()
            );
            let s = emb_stitch::stitch_object(o, Point::ZERO).unwrap();
            assert!(s.runs.iter().map(|r| r.points.len()).sum::<usize>() > 20);
        }
        // Object Properties shows the motif settings and changes nothing.
        for id in ids {
            h.st.selection.set(id);
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, crate::test_util::SCREEN)),
                ..Default::default()
            };
            let st = &mut h.st;
            let mut out = h.ctx.run_ui(input, |ui| {
                egui::CentralPanel::default().show(ui, |ui| crate::panels::properties(ui, st));
            });
            out.textures_delta.clear();
        }
        assert_eq!(h.st.doc.undo_len(), 2);
    }

    #[test]
    fn create_rename_and_delete_a_motif() {
        let dir = std::env::temp_dir().join(format!("emb-motifs-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let m = Motif::from_points(
            "Zig",
            &[Point::new(0.0, 0.0), Point::new(2.0, -2.0), Point::new(4.0, 0.0)],
        )
        .unwrap();
        let path = dir.join(file_name("Zig"));
        std::fs::write(&path, serde_json::to_string(&m).unwrap()).unwrap();
        let new = rename(&path, "Zag zig").unwrap();
        assert!(!path.exists() && new.exists());
        let back: Motif = serde_json::from_str(&std::fs::read_to_string(&new).unwrap()).unwrap();
        assert_eq!(back.name, "Zag zig");
        assert_eq!(back.points, m.points);
        std::fs::remove_dir_all(&dir).ok();
        assert_eq!(file_name("A/B:c"), "A_B_c.embmotif");
    }

    #[test]
    fn the_window_draws() {
        let mut h = Harness::empty();
        h.st.motifs_window.open = true;
        let before = h.st.doc.undo_len();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, crate::test_util::SCREEN)),
            ..Default::default()
        };
        let st = &mut h.st;
        let mut out = h.ctx.run_ui(input, |ui| window(ui.ctx(), st));
        out.textures_delta.clear();
        assert!(h.st.motifs_window.open);
        assert_eq!(h.st.doc.undo_len(), before);
    }
}
