//! Embroidery clipart (P8 Milestone 5): design pieces to use again. A
//! clipart item is a project file in the clipart folder (next to the
//! settings file): its objects, its threads, a name and keywords (in the
//! notes of the design). The design library engine (`emb_library`) finds
//! the items and makes their thumbnails.
//!
//! The Clipart tab shows the items with a search box. A double-click (or
//! Insert) puts a copy at the center of the view; a drag onto the canvas
//! puts it where it is dropped. Save as clipart keeps the selected objects
//! as a new item. The first start writes some built-in items.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use eframe::egui::{self, Color32, Rect, Sense, TextureHandle, Vec2, pos2, vec2};
use emb_geom::{BBox, Path as GeomPath, Point, Transform};
use emb_library::{Index, Query, SortKey};
use emb_model::{
    Design, DesignObject, FillParams, ObjectKind, Rgb, RunningParams, SatinGeometry, SatinParams, project,
};

use crate::app::AppState;
use crate::i18n::tr;

/// The size of a clipart card, in screen points.
const CARD: Vec2 = Vec2::new(96.0, 116.0);
const THUMB: f32 = 80.0;

/// The folder of the built-in items in the clipart folder.
pub const BUILTIN_FOLDER: &str = "Built-in";

/// The payload of a clipart drag from the Clipart tab to the canvas.
#[derive(Debug, Clone, PartialEq)]
pub struct ClipartDrag(pub PathBuf);

/// The Save as clipart window.
#[derive(Debug, Clone, Default)]
pub struct SaveClipart {
    pub name: String,
    pub keywords: String,
    pub message: String,
}

/// The state of the Clipart tab.
#[derive(Default)]
pub struct ClipartUi {
    pub index: Index,
    /// The clipart folder (`None`: no settings folder).
    pub dir: Option<PathBuf>,
    pub text: String,
    pub found: Vec<usize>,
    pub selected: Option<PathBuf>,
    pub save: Option<SaveClipart>,
    stale: bool,
    started: bool,
    thumbs: HashMap<String, Option<TextureHandle>>,
}

impl ClipartUi {
    fn cache(&self) -> Option<PathBuf> {
        self.dir.as_ref().map(|d| d.join(".index"))
    }
}

/// The clipart folder in the settings folder.
pub fn clipart_dir() -> Option<PathBuf> {
    crate::about::user_folder().map(|d| d.join("clipart"))
}

/// The built-in items: a name, keywords and the design.
pub fn builtins() -> Vec<(&'static str, &'static str, Design)> {
    let p = Point::new;
    let fill = |d: &mut Design, color: Rgb, name: &str, outer: GeomPath, angle: f64| {
        let t = d.add_thread(name, color);
        d.add_object(
            t,
            ObjectKind::Fill {
                outer,
                holes: Vec::new(),
                params: FillParams {
                    angle_deg: angle,
                    ..FillParams::default()
                },
            },
        );
    };
    let run = |d: &mut Design, color: Rgb, name: &str, path: GeomPath, repeats: u32| {
        let t = d.add_thread(name, color);
        d.add_object(
            t,
            ObjectKind::Running {
                path,
                params: RunningParams {
                    repeats,
                    ..RunningParams::default()
                },
            },
        );
    };
    let mut out = Vec::new();

    let mut heart = Design::new("Heart");
    let shape = GeomPath::new()
        .move_to(p(0.0, 14.0))
        .cubic_to(p(-8.0, 8.0), p(-20.0, 0.0), p(-20.0, -8.0))
        .cubic_to(p(-20.0, -16.0), p(-8.0, -20.0), p(0.0, -10.0))
        .cubic_to(p(8.0, -20.0), p(20.0, -16.0), p(20.0, -8.0))
        .cubic_to(p(20.0, 0.0), p(8.0, 8.0), p(0.0, 14.0))
        .close();
    fill(&mut heart, Rgb(200, 30, 50), "Red", shape.clone(), 45.0);
    run(&mut heart, Rgb(120, 10, 30), "Dark red", shape, 3);
    out.push(("Heart", "heart love valentine", heart));

    let mut star = Design::new("Star");
    let pts: Vec<Point> = (0..10)
        .map(|k| {
            let a = emb_geom::math::to_radians(-90.0 + 36.0 * f64::from(k));
            let r = if k % 2 == 0 { 25.0 } else { 10.0 };
            p(r * emb_geom::math::cos(a), r * emb_geom::math::sin(a))
        })
        .collect();
    fill(&mut star, Rgb(230, 170, 30), "Gold", GeomPath::polygon(&pts), 0.0);
    out.push(("Star", "star award sheriff", star));

    let mut ring = Design::new("Ring frame");
    let t = ring.add_thread("Navy", Rgb(25, 40, 100));
    ring.add_object(
        t,
        ObjectKind::Satin {
            geometry: SatinGeometry {
                rail_a: GeomPath::ellipse(Point::ZERO, 35.0, 35.0),
                rail_b: GeomPath::ellipse(Point::ZERO, 31.0, 31.0),
                rungs: Vec::new(),
            },
            params: SatinParams::default(),
        },
    );
    out.push(("Ring frame", "ring circle frame border badge", ring));

    let mut banner = Design::new("Banner");
    let ends = [
        p(-50.0, -9.0),
        p(50.0, -9.0),
        p(43.0, 0.0),
        p(50.0, 9.0),
        p(-50.0, 9.0),
        p(-43.0, 0.0),
    ];
    fill(&mut banner, Rgb(25, 40, 100), "Navy", GeomPath::polygon(&ends), 90.0);
    let inner = GeomPath::rect(BBox::new(p(-38.0, -6.0), p(38.0, 6.0)));
    run(&mut banner, Rgb(240, 240, 240), "White", inner, 1);
    out.push(("Banner", "banner ribbon scroll name", banner));

    let mut arrow = Design::new("Arrow");
    let a = [
        p(-30.0, -4.0),
        p(10.0, -4.0),
        p(10.0, -12.0),
        p(30.0, 0.0),
        p(10.0, 12.0),
        p(10.0, 4.0),
        p(-30.0, 4.0),
    ];
    fill(&mut arrow, Rgb(30, 120, 60), "Green", GeomPath::polygon(&a), 0.0);
    out.push(("Arrow", "arrow direction", arrow));

    let mut leaf = Design::new("Leaf");
    let shape = GeomPath::new()
        .move_to(p(0.0, 22.0))
        .cubic_to(p(-16.0, 10.0), p(-14.0, -12.0), p(0.0, -22.0))
        .cubic_to(p(14.0, -12.0), p(16.0, 10.0), p(0.0, 22.0))
        .close();
    fill(&mut leaf, Rgb(60, 150, 60), "Leaf green", shape, 60.0);
    let vein = GeomPath::new().move_to(p(0.0, 20.0)).line_to(p(0.0, -18.0));
    run(&mut leaf, Rgb(20, 80, 30), "Dark green", vein, 3);
    out.push(("Leaf", "leaf plant nature", leaf));

    let mut diamond = Design::new("Diamond");
    let d = [p(0.0, -20.0), p(15.0, 0.0), p(0.0, 20.0), p(-15.0, 0.0)];
    fill(&mut diamond, Rgb(40, 140, 200), "Blue", GeomPath::polygon(&d), 45.0);
    run(&mut diamond, Rgb(20, 40, 90), "Navy", GeomPath::polygon(&d), 3);
    out.push(("Diamond", "diamond gem shape", diamond));

    let mut wave = Design::new("Wave border");
    let pts: Vec<(Point, bool)> = (0..=24)
        .map(|k| {
            let x = -60.0 + 5.0 * f64::from(k);
            (
                p(
                    x,
                    6.0 * emb_geom::math::sin(emb_geom::math::to_radians(45.0 * f64::from(k))),
                ),
                true,
            )
        })
        .collect();
    run(
        &mut wave,
        Rgb(25, 40, 100),
        "Navy",
        GeomPath::through_points(&pts, false),
        3,
    );
    out.push(("Wave border", "wave border line edge", wave));
    for (_, keywords, d) in &mut out {
        d.notes = (*keywords).to_string();
    }
    out
}

/// Writes the built-in items into `dir/Built-in` if that folder is not
/// there (so an item that the user deleted does not come back while the
/// folder is there).
pub fn write_builtins(dir: &Path) -> std::io::Result<()> {
    let folder = dir.join(BUILTIN_FOLDER);
    if folder.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(&folder)?;
    for (name, _, d) in builtins() {
        project::save(&d, &folder.join(format!("{name}.{}", project::FILE_EXTENSION)))
            .map_err(std::io::Error::other)?;
    }
    Ok(())
}

impl AppState {
    /// At the first show of the tab: the built-in items and the scan.
    pub fn start_clipart(&mut self, dir: Option<PathBuf>) {
        self.clipart.started = true;
        self.clipart.dir = dir;
        if let Some(d) = self.clipart.dir.clone() {
            if let Err(e) = write_builtins(&d) {
                self.message = crate::i18n::fill("The built-in clipart is not written: {}", &[&e]);
            }
            if let Some(c) = self.clipart.cache() {
                self.clipart.index = Index::load(&c.join("index.json"));
            }
        }
        self.scan_clipart();
    }

    /// Reads the clipart folder again (only new and changed items are read).
    pub fn scan_clipart(&mut self) {
        let Some(dir) = self.clipart.dir.clone() else { return };
        let cache = self.clipart.cache();
        let thumbs = cache.as_ref().map(|c| c.join("thumbnails"));
        let (index, _) = emb_library::scan(&[dir], &self.clipart.index, thumbs.as_deref(), |_, _| true);
        if let Some(c) = cache {
            let _ = index.save(&c.join("index.json"));
        }
        self.clipart.index = index;
        self.clipart.stale = true;
    }

    fn refresh_clipart(&mut self) {
        if !std::mem::take(&mut self.clipart.stale) {
            return;
        }
        let q = Query::parse(&self.clipart.text, &self.hoops);
        let c = &mut self.clipart;
        let mut found: Vec<usize> = q
            .run(&c.index)
            .into_iter()
            .filter(|&i| c.index.entries[i].format == project::FILE_EXTENSION && c.index.entries[i].error.is_none())
            .collect();
        emb_library::sort(&c.index, &mut found, SortKey::Name, false);
        c.found = found;
    }

    /// Sets the search text of the Clipart tab.
    pub fn set_clipart_search(&mut self, text: &str) {
        self.clipart.text = text.to_string();
        self.clipart.stale = true;
        self.refresh_clipart();
    }

    /// Opens the Save as clipart window.
    pub fn open_save_clipart(&mut self) {
        let name = self
            .selected()
            .first()
            .and_then(|id| self.doc.design().object(*id))
            .and_then(|o| o.name.clone())
            .unwrap_or_default();
        self.clipart.save = Some(SaveClipart {
            name,
            ..Default::default()
        });
    }

    /// Saves the selected objects as the clipart item `name` (with its
    /// threads; the center of the objects goes to the origin).
    pub fn save_clipart(&mut self, name: &str, keywords: &str) -> Result<PathBuf, String> {
        let dir = self.clipart.dir.clone().ok_or("There is no clipart folder.")?;
        let name = crate::styles::valid_name(name).ok_or("Type a name without / \\ : * ? \" < > |")?;
        let ids = self.selected();
        if ids.is_empty() {
            return Err("Select the objects to keep as clipart.".into());
        }
        let path = dir.join(format!("{name}.{}", project::FILE_EXTENSION));
        if path.exists() {
            return Err(format!("A clipart item \"{name}\" is there already. Use another name."));
        }
        let src = self.doc.design();
        let objects: Vec<DesignObject> = ids
            .iter()
            .filter_map(|id| src.object(*id).map(|o| (*o).clone()))
            .collect();
        let mut d = Design::new(name);
        d.notes = keywords.trim().to_string();
        let new = d.paste_objects(&objects, &src.threads);
        d.center_objects_at(&new, Point::ZERO);
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        project::save(&d, &path).map_err(|e| e.to_string())?;
        self.scan_clipart();
        self.message = crate::i18n::fill("Saved \"{}\" as clipart ({} objects).", &[&name, &(objects.len())]);
        Ok(path)
    }

    /// Puts a copy of the clipart item at `at` (design coordinates; `None`:
    /// the center of the view) and selects it. One undo step.
    pub fn insert_clipart(&mut self, path: &Path, at: Option<Point>) {
        let item = match project::load(path) {
            Ok(d) => d,
            Err(e) => {
                self.message = crate::i18n::fill("Cannot read the clipart {}: {}", &[&(path.display()), &e]);
                return;
            }
        };
        let objects: Vec<DesignObject> = item.objects().iter().map(|o| (**o).clone()).collect();
        if objects.is_empty() {
            self.message = crate::i18n::fill("The clipart \"{}\" has no objects.", &[&item.name]);
            return;
        }
        let target = at.unwrap_or_else(|| {
            let inv = self
                .doc
                .design()
                .rotation_transform()
                .inverse()
                .unwrap_or(Transform::IDENTITY);
            inv.apply(self.view.center)
        });
        let ids = self.doc.edit("Insert clipart", |d| {
            let ids = d.paste_objects(&objects, &item.threads);
            d.center_objects_at(&ids, target);
            ids
        });
        self.message = crate::i18n::fill("Inserted the clipart \"{}\"", &[&item.name]);
        self.selection.set_all(ids);
    }
}

/// The Clipart tab.
pub fn panel(ui: &mut egui::Ui, st: &mut AppState) {
    if !st.clipart.started {
        st.start_clipart(clipart_dir());
    }
    st.refresh_clipart();
    ui.horizontal_wrapped(|ui| {
        let mut text = st.clipart.text.clone();
        ui.add(
            egui::TextEdit::singleline(&mut text)
                .hint_text(tr("Search clipart"))
                .desired_width(140.0),
        );
        if text != st.clipart.text {
            st.set_clipart_search(&text);
        }
        let any = !st.selection.is_empty();
        if ui
            .add_enabled(any, egui::Button::new(tr("Save selection…")))
            .on_hover_text(tr(
                "Keep the selected objects as a clipart item, with a name and keywords.",
            ))
            .clicked()
        {
            st.open_save_clipart();
        }
        if ui
            .button(egui_phosphor::regular::FOLDER_OPEN)
            .on_hover_text(tr("Show the clipart folder"))
            .clicked()
            && let Some(d) = &st.clipart.dir
        {
            crate::library::show_in_file_manager(d);
        }
        if ui
            .button(egui_phosphor::regular::ARROWS_CLOCKWISE)
            .on_hover_text(tr("Read the clipart folder again"))
            .clicked()
        {
            st.scan_clipart();
        }
    });
    ui.weak(tr("Double-click or drag onto the design."));
    ui.separator();
    if st.clipart.found.is_empty() {
        ui.weak(if st.clipart.index.entries.is_empty() {
            "No clipart yet. Select objects and click Save selection."
        } else {
            "No clipart matches the search."
        });
        return;
    }
    let ctx = ui.ctx().clone();
    let mut budget = 24;
    let found = st.clipart.found.clone();
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
            for i in found {
                card(ui, st, &ctx, i, &mut budget);
            }
        });
    });
    if budget == 0 {
        ctx.request_repaint();
    }
}

fn card(ui: &mut egui::Ui, st: &mut AppState, ctx: &egui::Context, i: usize, budget: &mut usize) {
    let e = st.clipart.index.entries[i].clone();
    let (rect, resp) = ui.allocate_exact_size(CARD, Sense::click_and_drag());
    let resp = resp.on_hover_text(if e.notes.is_empty() {
        e.name.clone()
    } else {
        format!("{}\n{}", e.name, e.notes)
    });
    if resp.drag_started() {
        resp.dnd_set_drag_payload(ClipartDrag(e.path.clone()));
    }
    if resp.clicked() {
        st.clipart.selected = Some(e.path.clone());
    }
    if resp.double_clicked() {
        st.insert_clipart(&e.path, None);
    }
    resp.context_menu(|ui| {
        if ui.button(tr("Insert")).clicked() {
            st.insert_clipart(&e.path, None);
            ui.close();
        }
        if ui.button(tr("Show in File Explorer")).clicked() {
            crate::library::show_in_file_manager(&e.path);
            ui.close();
        }
    });
    let v = ui.visuals().clone();
    let fill = if st.clipart.selected.as_ref() == Some(&e.path) {
        v.selection.bg_fill
    } else if resp.hovered() {
        v.widgets.hovered.weak_bg_fill
    } else {
        v.faint_bg_color
    };
    ui.painter().rect_filled(rect, 5.0, fill);
    let pic = Rect::from_center_size(
        pos2(rect.center().x, rect.min.y + 6.0 + THUMB / 2.0),
        Vec2::splat(THUMB),
    );
    let dir = st.clipart.cache().map(|c| c.join("thumbnails"));
    let tex = e
        .thumb
        .as_ref()
        .and_then(|t| crate::library::thumb(&mut st.clipart.thumbs, ctx, dir.as_deref(), t, budget));
    let p = ui.painter();
    if let Some(t) = tex {
        let s = t.size_vec2();
        let k = (THUMB / s.x.max(s.y)).min(1.0);
        p.image(
            t.id(),
            Rect::from_center_size(pic.center(), s * k),
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    }
    let mut job = egui::text::LayoutJob::single_section(
        e.name.clone(),
        egui::TextFormat {
            font_id: egui::FontId::proportional(12.0),
            color: v.text_color(),
            ..Default::default()
        },
    );
    job.wrap = egui::text::TextWrapping::truncate_at_width(rect.width() - 8.0);
    let g = p.layout_job(job);
    p.galley(
        pos2(rect.center().x - g.size().x / 2.0, pic.max.y + 6.0),
        g,
        v.text_color(),
    );
}

/// The Save as clipart window.
pub fn window(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut dlg) = st.clipart.save.take() else {
        return;
    };
    let mut open = true;
    let mut save = false;
    crate::dock::docked(
        egui::Window::new(tr("Save as Clipart")).id(egui::Id::new("Save as Clipart")),
        ctx,
        egui::Id::new("Save as Clipart"),
        st.canvas_rect,
    )
    .open(&mut open)
    .collapsible(false)
    .resizable(false)
    .show(ctx, |ui| {
        ui.label(crate::i18n::fill("{} selected objects", &[&(st.selection.len())]));
        egui::Grid::new("clipart-save").num_columns(2).show(ui, |ui| {
            ui.label(tr("Name"));
            ui.text_edit_singleline(&mut dlg.name);
            ui.end_row();
            ui.label(tr("Keywords"));
            ui.text_edit_singleline(&mut dlg.keywords).on_hover_text(tr(
                "Words that the search of the Clipart tab finds, for example: rose flower red",
            ));
            ui.end_row();
        });
        if !dlg.message.is_empty() {
            ui.colored_label(ui.visuals().warn_fg_color, &dlg.message);
        }
        if ui
            .add_enabled(!dlg.name.trim().is_empty(), egui::Button::new(tr("Save")))
            .clicked()
        {
            save = true;
        }
    });
    if save {
        match st.save_clipart(&dlg.name, &dlg.keywords) {
            Ok(_) => {
                st.focus_clipart = true;
                return;
            }
            Err(e) => dlg.message = e,
        }
    }
    if open {
        st.clipart.save = Some(dlg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::Harness;

    fn harness(name: &str) -> (Harness, PathBuf) {
        let dir = std::env::temp_dir().join(format!("emb-clipart-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut h = Harness::new();
        h.st.start_clipart(Some(dir.clone()));
        (h, dir)
    }

    #[test]
    fn the_built_in_items_are_written_once_and_found() {
        let (mut h, dir) = harness("builtin");
        assert_eq!(h.st.clipart.index.entries.len(), builtins().len());
        h.st.set_clipart_search("");
        assert_eq!(h.st.clipart.found.len(), 8);
        h.st.set_clipart_search("valentine");
        let names: Vec<&str> =
            h.st.clipart
                .found
                .iter()
                .map(|&i| h.st.clipart.index.entries[i].name.as_str())
                .collect();
        assert_eq!(names, ["Heart"], "keywords are in the notes");
        // Each item stitches.
        for e in &h.st.clipart.index.entries {
            assert!(e.stitches > 20 && e.thumb.is_some(), "{}: {}", e.name, e.stitches);
        }
        // A deleted built-in item does not come back while the folder is there.
        std::fs::remove_file(dir.join(BUILTIN_FOLDER).join("Star.embproj")).unwrap();
        h.st.start_clipart(Some(dir.clone()));
        assert_eq!(h.st.clipart.index.entries.len(), builtins().len() - 1);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn save_the_selection_and_insert_it_in_another_design() {
        let (mut h, dir) = harness("save");
        // The red fill and the white satin of the sample patch.
        let ids: Vec<_> = h.st.doc.design().objects().iter().take(2).map(|o| o.id).collect();
        h.st.selection.set_all(ids);
        assert!(h.st.save_clipart("", "x").is_err());
        let path = h.st.save_clipart("Patch", "round red patch").unwrap();
        assert!(
            h.st.save_clipart("Patch", "again")
                .unwrap_err()
                .contains("there already")
        );
        h.st.set_clipart_search("patch");
        assert_eq!(h.st.clipart.found.len(), 1);
        let saved = project::load(&path).unwrap();
        assert_eq!(saved.objects().len(), 2);
        let c = saved
            .bbox_of(&saved.objects().iter().map(|o| o.id).collect::<Vec<_>>())
            .unwrap()
            .center();
        assert!(c.length() < 1e-6, "the item is centered on the origin: {c:?}");

        // Into an empty design at a point: one undo step, the threads come too.
        h.st.new_builtin_design();
        h.st.insert_clipart(&path, Some(Point::new(30.0, -20.0)));
        let d = h.st.doc.design();
        assert_eq!(d.objects().len(), 2);
        assert_eq!(h.st.selection.len(), 2);
        let b = d
            .bbox_of(&d.objects().iter().map(|o| o.id).collect::<Vec<_>>())
            .unwrap();
        assert!(b.center().distance(Point::new(30.0, -20.0)) < 1e-6);
        assert!(d.threads.iter().any(|t| t.name == "Red"));
        h.st.doc.undo();
        assert!(h.st.doc.design().objects().is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_tab_and_the_window_draw() {
        let (mut h, dir) = harness("draw");
        h.st.select_all();
        h.st.open_save_clipart();
        let ctx = egui::Context::default();
        for _ in 0..2 {
            let mut out = ctx.run_ui(Default::default(), |ui| {
                egui::CentralPanel::default().show(ui, |ui| panel(ui, &mut h.st));
                window(&ui.ctx().clone(), &mut h.st);
            });
            out.textures_delta.clear();
        }
        assert!(!h.st.clipart.thumbs.is_empty(), "thumbnails are read");
        std::fs::remove_dir_all(&dir).ok();
    }
}

#[cfg(test)]
mod drop_tests {
    use super::*;
    use crate::test_util::Harness;
    use eframe::egui::{Event, Modifiers, PointerButton};

    /// A drag from the Clipart tab released over the canvas puts the item
    /// at the drop point.
    #[test]
    fn a_drop_on_the_canvas_inserts_at_the_point() {
        let dir = std::env::temp_dir().join(format!("emb-clipart-drop-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut h = Harness::empty();
        h.st.start_clipart(Some(dir.clone()));
        let star = dir.join(BUILTIN_FOLDER).join("Star.embproj");
        let at = h.screen(Point::new(40.0, 30.0));
        let ev = |pressed| Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        h.frame(vec![Event::PointerMoved(at)]);
        h.frame(vec![ev(true)]);
        egui::DragAndDrop::set_payload(&h.ctx, ClipartDrag(star));
        h.frame(vec![ev(false)]);
        let d = h.st.doc.design();
        assert_eq!(d.objects().len(), 1, "{}", h.st.message);
        let b = d.bbox_of(&[d.objects()[0].id]).unwrap();
        assert!(b.center().distance(Point::new(40.0, 30.0)) < 0.5, "{:?}", b.center());
        std::fs::remove_dir_all(&dir).ok();
    }
}
