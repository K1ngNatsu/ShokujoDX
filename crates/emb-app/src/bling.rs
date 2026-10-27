//! Rhinestones in the app (P9 Milestone 3, ADR 0039): the stone tools,
//! the stones on the canvas, Object Properties and Convert to rhinestones.

use std::cell::RefCell;
use std::sync::Arc;

use eframe::egui::{self, Color32, Rect, Stroke};
use emb_geom::Point;
use emb_model::bling::{BlingFill, BlingLayout, PlacedStone, STONE_COLORS, STONE_SIZES, Stone};
use emb_model::{Bling, ObjectId, ObjectKind, Rgb};

use crate::a11y::Named;
use crate::app::AppState;
use crate::canvas::color32;
use crate::draw::{Tool, thread_for_new};
use crate::i18n::tr;
use crate::manipulate::ViewMap;

/// The stones of the design, made again only when a rhinestone object
/// changes (a key of their ids and revisions).
#[derive(Default)]
pub struct StoneCache(RefCell<Option<(u64, Arc<Vec<PlacedStone>>)>>);

impl std::fmt::Debug for StoneCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("StoneCache")
    }
}

fn bling_of(kind: &ObjectKind) -> Option<&Bling> {
    match kind {
        ObjectKind::Bling(b) => Some(b),
        _ => None,
    }
}

impl AppState {
    /// All stones of the design (`Design::stones`), from the cache.
    pub fn placed_stones(&self) -> Arc<Vec<PlacedStone>> {
        use std::hash::{Hash, Hasher};
        let d = self.doc.design();
        let mut h = std::collections::hash_map::DefaultHasher::new();
        for o in d.objects().iter().filter(|o| matches!(o.kind, ObjectKind::Bling(_))) {
            (o.id, o.revision).hash(&mut h);
        }
        let key = h.finish();
        if let Some((k, s)) = self.stone_cache.0.borrow().as_ref()
            && *k == key
        {
            return s.clone();
        }
        let s = Arc::new(d.stones());
        *self.stone_cache.0.borrow_mut() = Some((key, s.clone()));
        s
    }

    /// Changes the selected objects into rhinestones (see [`ToStones`]).
    /// One undo step.
    pub fn convert_to_stones(&mut self, how: ToStones) {
        let d = self.doc.design();
        let defaults = self.settings.defaults.clone();
        let current = d.current.clone();
        let make = move |k: &ObjectKind| -> Option<ObjectKind> {
            let area = emb_model::shaping::takes_part(k) || matches!(k, ObjectKind::Lettering(_));
            if how != ToStones::Outline && !area {
                return None;
            }
            let Some(ObjectKind::Vector { path, .. }) = emb_model::vector::to_vector(k, Rgb(0, 0, 0)) else {
                return None;
            };
            let layout = match how {
                ToStones::Outline => BlingLayout::Run { path },
                ToStones::CenterLine => BlingLayout::Run {
                    path: center_lines(&path, &current)?,
                },
                ToStones::Fill => BlingLayout::Fill {
                    outer: path,
                    holes: Vec::new(),
                    fill: defaults.stone_fill,
                },
            };
            Some(ObjectKind::Bling(Box::new(Bling {
                gap_mm: defaults.stone_gap_mm,
                ..Bling::new(defaults.stone.clone(), layout)
            })))
        };
        let ids: Vec<ObjectId> = self
            .selection
            .in_sew_order(d)
            .into_iter()
            .filter(|id| d.object(*id).and_then(|o| make(&o.kind)).is_some())
            .collect();
        if ids.is_empty() {
            self.message = match how {
                ToStones::Outline => "Select objects with an outline or a line to change into a rhinestone run.",
                _ => "Select a fill, a satin, an appliqué, lettering or a closed vector.",
            }
            .into();
            return;
        }
        if self.refuse_linked(&ids) {
            return;
        }
        let n = ids.len();
        self.doc.edit("Convert to rhinestones", |d| {
            for id in ids {
                if let Some(o) = d.object_mut(id)
                    && let Some(k) = make(&o.kind)
                {
                    o.kind = k;
                }
            }
        });
        self.message = crate::i18n::fill(
            "{} objects are rhinestones now. Set the stone in Object Properties.",
            &[&n],
        );
    }
}

/// What Convert to rhinestones makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToStones {
    /// A run along the outlines (and along lines).
    Outline,
    /// A run along the center lines of the areas (narrow shapes and
    /// lettering: one line of stones in each stroke).
    CenterLine,
    /// A fill of the areas.
    Fill,
}

/// The center lines of the closed shapes of `path` (even-odd), as one
/// path; `None` if there are none.
pub fn center_lines(path: &emb_geom::Path, current: &emb_model::CurrentProperties) -> Option<emb_geom::Path> {
    let subs: Vec<emb_geom::Path> = path.subpaths().into_iter().map(emb_geom::Path::closed).collect();
    let lines: Vec<emb_geom::Path> = emb_geom::region::nest(&subs)
        .iter()
        .flat_map(|(o, hs)| emb_art::click::objects(o, hs, emb_art::click::ClickStitch::CenterLine, current))
        .filter_map(|k| match k {
            ObjectKind::Running { path, .. } => Some(path),
            _ => None,
        })
        .collect();
    (!lines.is_empty()).then(|| emb_geom::Path::joined(&lines))
}

/// The Stone tool: each click puts a stone. The stones go into the
/// selected object of single stones (with the same stone), else into a new
/// one.
pub fn tool_input(response: &egui::Response, st: &mut AppState, map: &ViewMap) {
    if !response.clicked() {
        return;
    }
    let Some(pos) = response.interact_pointer_pos() else {
        return;
    };
    let p = st.snap(map, map.to_design(pos));
    add_stone(st, p);
}

/// Adds a single stone at `p` (see [`tool_input`]).
pub fn add_stone(st: &mut AppState, p: Point) {
    let stone = st.settings.defaults.stone.clone();
    let target = st.selection.single().filter(|id| {
        st.doc
            .design()
            .object(*id)
            .and_then(|o| bling_of(&o.kind))
            .is_some_and(|b| matches!(b.layout, BlingLayout::Stones { .. }) && b.stone == stone)
    });
    let current = st.current_thread;
    let id = st.doc.edit("Add rhinestone", |d| match target {
        Some(id) => {
            if let Some(ObjectKind::Bling(b)) = d.object_mut(id).map(|o| &mut o.kind)
                && let BlingLayout::Stones { points } = &mut b.layout
            {
                points.push(p);
            }
            id
        }
        None => {
            let t = thread_for_new(d, current);
            d.add_object(
                t,
                ObjectKind::Bling(Box::new(Bling::new(stone, BlingLayout::Stones { points: vec![p] }))),
            )
        }
    });
    st.selection.set(id);
    st.message = tr("Stone added. Click for the next one, or press Esc.").into();
}

/// Draws the stones: a disk in the stone color with a light spot, and a
/// red ring where stones overlap.
pub fn draw(painter: &egui::Painter, rect: Rect, st: &AppState) {
    let stones = st.placed_stones();
    if stones.is_empty() {
        return;
    }
    let design = st.doc.design();
    let t = design.rotation_transform();
    let scale = st.view.scale() as f32;
    let shown = |id: ObjectId| design.object(id).is_some_and(|o| st.view_filter.shows(o));
    let accent = Color32::from_rgb(80, 160, 255);
    for s in stones.iter().filter(|s| shown(s.object)) {
        let c = st.view.to_screen(rect, t.apply(s.at));
        let r = s.radius as f32 * scale;
        if !rect.expand(r).contains(c) {
            continue;
        }
        let fill = color32(s.color);
        // A dark stone gets a light edge, so it shows on a dark fabric.
        let light = u32::from(s.color.0) + u32::from(s.color.1) + u32::from(s.color.2) > 240;
        let edge = if light {
            fill.gamma_multiply(0.55)
        } else {
            Color32::from_gray(150)
        };
        painter.circle(c, r, fill, Stroke::new((r * 0.08).clamp(0.5, 1.5), edge));
        if r > 3.0 {
            painter.circle_filled(
                c + egui::vec2(-r * 0.3, -r * 0.3),
                r * 0.25,
                Color32::from_white_alpha(150),
            );
        }
        if s.overlaps {
            painter.circle_stroke(c, r + 1.5, Stroke::new(2.0, Color32::from_rgb(230, 40, 40)));
        }
        if st.selection.contains(s.object) {
            painter.circle_stroke(c, r + 1.0, Stroke::new(1.5, accent));
        }
    }
}

/// A choice of stone: size and color. Returns `true` if it changed.
/// The most stones in the user's palette.
pub const MAX_MY_STONES: usize = 24;

/// The stone picker: the size, the color, any color, and the user's own
/// stones (`mine`, P10 Milestone 6: keep the stone, pick one, remove one).
pub fn stone_ui(ui: &mut egui::Ui, salt: &str, stone: &mut Stone, mine: &mut Vec<Stone>) -> bool {
    let mut changed = false;
    egui::ComboBox::from_id_salt((salt, "size"))
        .selected_text(format!("SS{} ({:.1} mm)", stone.ss, stone.diameter_mm()))
        .show_ui(ui, |ui| {
            for (ss, mm) in STONE_SIZES {
                changed |= ui
                    .selectable_value(&mut stone.ss, ss, format!("SS{ss} ({mm:.1} mm)"))
                    .changed();
            }
        })
        .response
        .on_hover_text(tr("The stone size: SS (stone size) number and diameter"));
    egui::ComboBox::from_id_salt((salt, "color"))
        .selected_text(stone.name.clone())
        .show_ui(ui, |ui| {
            for (name, rgb) in STONE_COLORS {
                let on = stone.name == name && stone.color == rgb;
                let resp = ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                    ui.painter().circle_filled(r.center(), 6.0, color32(rgb));
                    ui.selectable_label(on, name)
                });
                if resp.inner.clicked() {
                    stone.name = name.into();
                    stone.color = rgb;
                    changed = true;
                }
            }
        });
    let mut c = [stone.color.0, stone.color.1, stone.color.2];
    if ui
        .color_edit_button_srgb(&mut c)
        .on_hover_text(tr("Any stone color"))
        .changed()
    {
        stone.color = Rgb(c[0], c[1], c[2]);
        if !STONE_COLORS.iter().any(|(_, rgb)| *rgb == stone.color) {
            stone.name = format!("Color {}", stone.color.to_hex());
        }
        changed = true;
    }
    let mut remove = None;
    egui::ComboBox::from_id_salt((salt, "mine"))
        .selected_text(tr("My stones"))
        .show_ui(ui, |ui| {
            if mine.is_empty() {
                ui.weak(tr("No stones yet. Keep the stone that you use often."));
            }
            for (i, s) in mine.iter().enumerate() {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                    ui.painter().circle_filled(r.center(), 6.0, color32(s.color));
                    if ui.selectable_label(s == stone, s.label()).clicked() {
                        *stone = s.clone();
                        changed = true;
                    }
                    if ui
                        .small_button("×")
                        .on_hover_text(tr("Remove from my stones"))
                        .named(tr("Remove from my stones"))
                        .clicked()
                    {
                        remove = Some(i);
                    }
                });
            }
            ui.separator();
            let kept = mine.contains(stone);
            if ui
                .add_enabled(
                    !kept && mine.len() < MAX_MY_STONES,
                    egui::Button::new(tr("Keep this stone")),
                )
                .on_hover_text(tr("Put the stone (size and color) in my stones, for all designs."))
                .clicked()
            {
                mine.push(stone.clone());
            }
        })
        .response
        .on_hover_text(tr("Your own stones: sizes and colors that you use often"));
    if let Some(i) = remove {
        mine.remove(i);
    }
    changed
}

fn gap_ui(ui: &mut egui::Ui, gap: &mut f64) -> bool {
    ui.label(tr("Gap"));
    ui.add(
        egui::DragValue::new(gap)
            .range(0.0..=10.0)
            .speed(0.05)
            .max_decimals(2)
            .clamp_existing_to_range(false)
            .suffix(" mm"),
    )
    .on_hover_text(tr("The space between the edges of two stones next to each other"))
    .changed()
}

fn fill_ui(ui: &mut egui::Ui, fill: &mut BlingFill) -> bool {
    let mut changed = false;
    let rows = matches!(fill, BlingFill::Rows { .. });
    if ui.selectable_label(rows, tr("Rows")).clicked() && !rows {
        *fill = BlingFill::Rows { angle_deg: 0.0 };
        changed = true;
    }
    if ui.selectable_label(!rows, tr("Rings")).clicked() && rows {
        *fill = BlingFill::Rings;
        changed = true;
    }
    if let BlingFill::Rows { angle_deg } = fill {
        changed |= ui
            .add(
                egui::DragValue::new(angle_deg)
                    .range(0.0..=179.0)
                    .clamp_existing_to_range(false)
                    .suffix("°"),
            )
            .on_hover_text(tr(
                "The angle of the rows; each other row moves half a stone (a honeycomb)",
            ))
            .changed();
    }
    changed
}

/// The clearance of the template holes. Returns `true` if it changed.
pub fn clearance_ui(ui: &mut egui::Ui, mm: &mut f64) -> bool {
    ui.label(tr("Template holes +"));
    ui.add(
        egui::DragValue::new(mm)
            .range(0.0..=3.0)
            .speed(0.01)
            .max_decimals(2)
            .clamp_existing_to_range(false)
            .suffix(" mm"),
    )
    .on_hover_text(tr(
        "How much larger a hole of the template is than its stone. Thick or soft template \
         material needs more; 0.2 to 0.4 mm is usual.",
    ))
    .changed()
}

impl AppState {
    /// Adds the sewn template marks (or puts them back at the corners of
    /// the stones). One undo step.
    pub fn add_template_marks(&mut self) {
        let mut test = self.doc.design().clone();
        if test.add_template_marks().is_none() {
            self.message = tr("The design has no rhinestones.").into();
            return;
        }
        let id = self.doc.edit("Template Marks", |d| d.add_template_marks());
        if let Some(id) = id {
            self.selection.set(id);
        }
        self.message = tr(
            "The last color sews four crosses: the marks to put the rhinestone template in place. \
                        Pull them out after the press.",
        )
        .into();
    }

    /// Asks for a file name and writes the rhinestone template: SVG or DXF
    /// for a cutter, or PDF (the key and the template at full size).
    pub fn stone_template_dialog(&mut self) {
        let clearance = self.settings.output.stone_clearance_mm;
        let Some(t) = self.doc.design().stone_template(clearance) else {
            self.message = tr("The design has no rhinestones.").into();
            return;
        };
        let name = format!("{} template.svg", self.doc.design().name);
        let Some(p) = rfd::FileDialog::new()
            .set_title("Rhinestone template")
            .add_filter("SVG (cutting machines)", &["svg"])
            .add_filter("DXF (cutter and laser programs)", &["dxf"])
            .add_filter("PDF (on paper, at full size)", &["pdf"])
            .set_directory(self.dialog_dir())
            .set_file_name(name)
            .save_file()
        else {
            return;
        };
        match self.write_stone_template(&t, &p) {
            Ok(()) => {
                self.message = crate::i18n::fill(
                    "Wrote the template ({} holes, {} mm larger than the stones) to {}",
                    &[&(t.holes.len()), &format!("{:.2}", clearance), &(p.display())],
                )
            }
            Err(e) => {
                self.notice = Some(crate::file::Notice {
                    title: "Template not written".into(),
                    lines: vec![e],
                })
            }
        }
    }

    /// Writes the template `t` to `p` (SVG, DXF or PDF by the extension).
    pub fn write_stone_template(&self, t: &emb_model::bling::StoneTemplate, p: &std::path::Path) -> Result<(), String> {
        let ext = p
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        let name = self.doc.design().name.clone();
        let bytes = match ext.as_str() {
            "dxf" => emb_formats::stones::dxf(t).into_bytes(),
            "pdf" => {
                let spec = emb_render::stone_sheet::StoneSheetSpec {
                    name: name.clone(),
                    paper: self.settings.output.paper,
                    clearance_mm: self.settings.output.stone_clearance_mm,
                };
                emb_render::pdf::write(&emb_render::stone_sheet::pages(t, &spec), &name)
            }
            _ => emb_formats::stones::svg(t).into_bytes(),
        };
        std::fs::write(p, bytes).map_err(|e| format!("{}: {e}", p.display()))
    }
}

/// The property bar of the stone tools: the stone, the gap, and the fill.
pub fn bar(
    ui: &mut egui::Ui,
    stone: &mut Stone,
    mine: &mut Vec<Stone>,
    gap: &mut f64,
    fill: &mut BlingFill,
    tool: Tool,
) {
    stone_ui(ui, "bar_stone", stone, mine);
    if tool != Tool::Stone {
        gap_ui(ui, gap);
    }
    if tool == Tool::StoneFill {
        fill_ui(ui, fill);
    }
}

/// Object Properties of a rhinestone object.
pub fn properties(ui: &mut egui::Ui, st: &mut AppState, id: ObjectId, b: &Bling) {
    let all = b.centers().len();
    let placed = st.placed_stones();
    let here: Vec<&PlacedStone> = placed.iter().filter(|s| s.object == id).collect();
    let overlaps = here.iter().filter(|s| s.overlaps).count();
    ui.label(crate::i18n::fill(
        "{} stones{}{}",
        &[
            &(here.len()),
            &(if all > here.len() {
                format!(" ({} left out: they overlap stones before them)", all - here.len())
            } else {
                String::new()
            }),
            &(if overlaps > 0 {
                format!("; {overlaps} overlap other stones (red)")
            } else {
                String::new()
            }),
        ],
    ));
    let mut n = b.clone();
    let mut changed = false;
    let mut mine = st.settings.my_stones.clone();
    ui.horizontal(|ui| {
        ui.label(tr("Stone"));
        changed |= stone_ui(ui, "props_stone", &mut n.stone, &mut mine);
    });
    if mine != st.settings.my_stones {
        let mut s = st.settings.clone();
        s.my_stones = mine;
        st.set_settings(s);
    }
    if !matches!(n.layout, BlingLayout::Stones { .. }) {
        ui.horizontal(|ui| changed |= gap_ui(ui, &mut n.gap_mm));
    }
    if let BlingLayout::Fill { fill, .. } = &mut n.layout {
        ui.horizontal(|ui| {
            ui.label(tr("Fill"));
            changed |= fill_ui(ui, fill);
        });
    }
    changed |= ui
        .checkbox(&mut n.remove_overlaps, tr("Remove Overlaps"))
        .on_hover_text(tr(
            "Leave out the stones that overlap stones placed before them (earlier in the sew order).",
        ))
        .changed();
    if changed && n != *b {
        st.doc.edit_merged(
            "Change rhinestones",
            emb_model::MergeKey::new(format!("{}:bling", id.0)),
            |d| {
                if let Some(o) = d.object_mut(id) {
                    o.kind = ObjectKind::Bling(Box::new(n));
                }
            },
        );
        if !ui.input(|i| i.pointer.any_down()) {
            st.doc.end_merge();
        }
    }
    ui.separator();
    let single = matches!(b.layout, BlingLayout::Stones { .. });
    ui.weak(if single {
        "Rhinestones sew nothing. Reshape moves each stone; Delete in Reshape removes one."
    } else {
        "Rhinestones sew nothing. Reshape changes the line or the shape; the stones follow. To single stones (Specialty tab) lets you move each stone."
    });
}

/// A short description for the Sequence panel.
pub fn detail(b: &Bling) -> String {
    let kind = match b.layout {
        BlingLayout::Stones { .. } => "single stones",
        BlingLayout::Run { .. } => "run",
        BlingLayout::Fill { .. } => "fill",
    };
    format!("Rhinestones · {} · {kind} · no stitches", b.stone.label())
}

#[cfg(test)]
mod tests {
    use eframe::egui::{Key, PointerButton};
    use emb_geom::{BBox, Path};

    use super::*;
    use crate::test_util::Harness;

    #[test]
    fn the_run_tool_and_the_stone_tool() {
        let mut h = Harness::empty();
        h.st.set_tool(Tool::StoneRun);
        let map = h.map();
        for x in [0.0, 30.0] {
            h.click(PointerButton::Primary, map.to_screen(Point::new(x, 0.0)));
        }
        h.key(Key::Enter);
        let d = h.st.doc.design();
        assert_eq!(d.objects().len(), 1);
        let b = bling_of(&d.objects()[0].kind).expect("a rhinestone run");
        assert!(matches!(b.layout, BlingLayout::Run { .. }));
        assert_eq!(h.st.placed_stones().len(), 10);
        // Single stones: two clicks, one object.
        h.st.set_tool(Tool::Stone);
        h.st.selection.clear();
        for x in [0.0, 10.0] {
            h.click(PointerButton::Primary, map.to_screen(Point::new(x, 20.0)));
        }
        let d = h.st.doc.design();
        assert_eq!(d.objects().len(), 2);
        let b = bling_of(&d.objects()[1].kind).unwrap();
        assert!(matches!(&b.layout, BlingLayout::Stones { points } if points.len() == 2));
        // The rhinestones sew nothing.
        let plan = emb_stitch::build_plan(d, &Default::default(), &mut Default::default());
        assert_eq!(plan.stitches.stitch_count(), 0);
    }

    #[test]
    fn convert_makes_runs_and_fills() {
        let mut h = Harness::empty();
        let id = h.st.doc.edit("t", |d| {
            let t = d.add_thread("Red", Rgb(200, 0, 0));
            d.add_object(
                t,
                ObjectKind::Fill {
                    outer: Path::rect(BBox::new(Point::new(0.0, 0.0), Point::new(30.0, 20.0))),
                    holes: Vec::new(),
                    params: Default::default(),
                },
            )
        });
        h.st.selection.set(id);
        h.st.convert_to_stones(ToStones::Fill);
        let d = h.st.doc.design();
        let b = bling_of(&d.objects()[0].kind).expect("rhinestones");
        assert!(matches!(b.layout, BlingLayout::Fill { .. }));
        let n = h.st.placed_stones().len();
        assert!(n > 30, "{n}");
        h.st.doc.undo();
        h.st.selection.set(id);
        h.st.convert_to_stones(ToStones::Outline);
        let d = h.st.doc.design();
        assert!(matches!(
            bling_of(&d.objects()[0].kind).unwrap().layout,
            BlingLayout::Run { .. }
        ));
        // About 100 mm round the box: 30 stones of 3.3 mm.
        let n = h.st.placed_stones().len();
        assert!((28..=31).contains(&n), "{n}");
        // A bar 3 mm wide: one line of stones along its middle.
        let bar = h.st.doc.edit("t", |d| {
            let t = d.threads[0].id;
            d.add_object(
                t,
                ObjectKind::Fill {
                    outer: Path::rect(BBox::new(Point::new(0.0, 40.0), Point::new(40.0, 43.0))),
                    holes: Vec::new(),
                    params: Default::default(),
                },
            )
        });
        h.st.selection.set(bar);
        h.st.convert_to_stones(ToStones::CenterLine);
        let s: Vec<PlacedStone> =
            h.st.placed_stones()
                .iter()
                .filter(|s| s.object == bar)
                .copied()
                .collect();
        assert!(s.len() >= 10, "{}", s.len());
        assert!(s.iter().all(|s| (s.at.y - 41.5).abs() < 0.3), "{s:?}");
    }

    #[test]
    fn the_template_files_have_a_hole_for_each_stone() {
        let mut h = Harness::empty();
        add_stone(&mut h.st, Point::ZERO);
        add_stone(&mut h.st, Point::new(10.0, 0.0));
        let t = h.st.doc.design().stone_template(0.3).unwrap();
        let dir = std::env::temp_dir().join(format!("emb-stones-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for ext in ["svg", "dxf", "pdf"] {
            let p = dir.join(format!("t.{ext}"));
            h.st.write_stone_template(&t, &p).unwrap();
            let text = String::from_utf8_lossy(&std::fs::read(&p).unwrap()).to_string();
            match ext {
                "svg" => assert_eq!(text.matches("<circle").count(), 2),
                "dxf" => assert_eq!(text.matches("CIRCLE\r\n").count(), 2),
                _ => assert!(text.starts_with("%PDF")),
            }
        }
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn template_marks_sew_and_go_in_the_output() {
        let mut h = Harness::empty();
        add_stone(&mut h.st, Point::ZERO);
        add_stone(&mut h.st, Point::new(20.0, 0.0));
        h.st.add_template_marks();
        let d = h.st.doc.design();
        assert!(d.sewn_mark_centers().is_some());
        let plan = emb_stitch::build_plan(d, &Default::default(), &mut Default::default());
        assert!(plan.stitches.stitch_count() > 10, "the marks sew");
        let dir = std::env::temp_dir().join(format!("emb-mixed-{}", std::process::id()));
        let files = crate::output::write_files(
            d,
            &plan,
            h.st.catalogs(),
            &crate::output::OutputProfile::default(),
            &dir,
            "mixed",
        )
        .unwrap();
        let names: Vec<String> = files
            .iter()
            .filter_map(|f| f.file_name().map(|n| n.to_string_lossy().into_owned()))
            .collect();
        assert!(names.contains(&"mixed template.svg".to_string()), "{names:?}");
        assert!(names.contains(&"mixed template.pdf".to_string()));
        assert!(names.contains(&"mixed worksheet.pdf".to_string()));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_cache_follows_changes() {
        let mut h = Harness::empty();
        h.st.settings.defaults.stone.ss = 20;
        add_stone(&mut h.st, Point::ZERO);
        assert_eq!(h.st.placed_stones().len(), 1);
        add_stone(&mut h.st, Point::new(2.0, 0.0));
        let s = h.st.placed_stones();
        // The second stone overlaps the first one and is left out.
        assert_eq!(s.len(), 1);
        h.st.doc.undo();
        add_stone(&mut h.st, Point::new(10.0, 0.0));
        assert_eq!(h.st.placed_stones().len(), 2);
    }
}
