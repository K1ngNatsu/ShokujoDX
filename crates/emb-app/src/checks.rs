//! The check views of the surface (P18 view plan, Milestone 8, ADR 0129):
//! View > Fabric Shows Through and View > High Thread color the places of
//! `emb_render::check` over the canvas, with a line that counts them. A
//! thread makes the map from the lay of the tiles (canvas coordinates) for
//! each new lay.

use std::sync::{Arc, mpsc};

use eframe::egui::{self, Color32, FontId, Mesh, Pos2, Rect, TextureHandle, TextureOptions};
use emb_geom::Point;
use emb_render::check::CheckMap;
#[cfg(test)]
use emb_render::check::Place;

use crate::app::AppState;
use crate::i18n::{fill, tr};

/// The check map of the lay that shows, made on a thread.
#[derive(Default)]
pub struct CheckCache {
    /// The key of the lay and of the fabric of the map.
    key: Option<u64>,
    map: Option<Arc<CheckMap>>,
    job: Option<(u64, mpsc::Receiver<Option<CheckMap>>)>,
    /// The picture of the map for the checks that show and their limits.
    texture: Option<(u64, TextureHandle)>,
}

impl AppState {
    /// The check map of the design that shows, if it is made (a thread
    /// makes it for each new lay).
    pub fn check_map(&mut self, ctx: &egui::Context) -> Option<Arc<CheckMap>> {
        let (lay, laid) = crate::stitch_tiles::laid_of(ctx, self)?;
        let d = self.doc.design();
        let ground = self.ground_of(d);
        let key = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            (lay, ground.as_ref().map(emb_render::fabric::Ground::key)).hash(&mut h);
            h.finish()
        };
        let c = &mut self.check_cache;
        if let Some((k, rx)) = &c.job {
            match rx.try_recv() {
                Ok(map) => {
                    c.key = Some(*k);
                    c.map = map.map(Arc::new);
                    c.texture = None;
                    c.job = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => c.job = None,
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        if c.key != Some(key) && c.job.as_ref().is_none_or(|(k, _)| *k != key) {
            // The areas in the coordinates of the canvas (as the lay).
            let back = d.from_hoop_transform();
            let areas: Vec<Vec<Vec<Point>>> = emb_render::check::areas(d)
                .into_iter()
                .map(|rings| {
                    rings
                        .into_iter()
                        .map(|r| r.into_iter().map(|p| back.apply(p)).collect())
                        .collect()
                })
                .collect();
            let (tx, rx) = mpsc::channel();
            let repaint = ctx.clone();
            let work = move || {
                let _ = tx.send(CheckMap::of(&laid, &areas, ground.as_ref()));
                repaint.request_repaint();
            };
            if self.stitch_tiles.sync {
                work();
            } else if std::thread::Builder::new()
                .name("surface-check".into())
                .stack_size(crate::resources::JOB_STACK)
                .spawn(work)
                .is_err()
            {
                return c.map.clone();
            }
            c.job = Some((key, rx));
            if self.stitch_tiles.sync {
                return self.check_map(ctx);
            }
        }
        // The map before shows until the new one is there.
        self.check_cache.map.clone()
    }

    /// The places of the checks: the fabric that shows through, and the
    /// high thread.
    #[cfg(test)]
    pub fn check_places(&mut self, ctx: &egui::Context) -> Option<(Vec<Place>, Vec<Place>)> {
        let map = self.check_map(ctx)?;
        let s = &self.settings;
        Some((
            map.show_through(s.show_through_limit),
            map.high(s.thread_height_limit_mm),
        ))
    }
}

/// The color of a cell: the fabric that shows through (blue to magenta),
/// and the high thread (orange to red).
fn color(map: &CheckMap, i: usize, through: Option<f64>, high: Option<f64>) -> Color32 {
    if let Some(limit) = high {
        let v = f64::from(map.thread[i]);
        if v > limit {
            return Color32::from_rgba_unmultiplied(230, 30, 30, 200);
        }
        if v > limit * 0.85 {
            return Color32::from_rgba_unmultiplied(250, 150, 30, 140);
        }
    }
    if let Some(limit) = through {
        let v = f64::from(map.open[i]);
        if v > limit {
            return Color32::from_rgba_unmultiplied(220, 40, 220, 200);
        }
        if v > limit * 0.6 {
            return Color32::from_rgba_unmultiplied(80, 120, 255, 110);
        }
    }
    Color32::TRANSPARENT
}

/// Draws the checks that are on over the stitches, with a line that counts
/// the places.
pub fn draw(painter: &egui::Painter, ctx: &egui::Context, rect: Rect, st: &mut AppState) {
    if !(st.show_through || st.show_high_thread) {
        return;
    }
    let through = st.show_through.then_some(st.settings.show_through_limit);
    let high = st.show_high_thread.then_some(st.settings.thread_height_limit_mm);
    let Some(map) = st.check_map(ctx) else {
        status(painter, ctx, rect, st, &[tr("Checking the surface…").to_string()]);
        ctx.request_repaint_after(crate::clock::Duration::from_millis(60));
        return;
    };
    let tkey = {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        (
            Arc::as_ptr(&map) as usize,
            through.map(f64::to_bits),
            high.map(f64::to_bits),
        )
            .hash(&mut h);
        h.finish()
    };
    let cache = &mut st.check_cache;
    let texture = match &cache.texture {
        Some((k, t)) if *k == tkey => t.clone(),
        _ => {
            let pixels = (0..map.cols * map.rows)
                .map(|i| color(&map, i, through, high))
                .collect();
            let image = egui::ColorImage::new([map.cols, map.rows], pixels);
            let t = crate::resources::load_texture(ctx, "surface-check", image, TextureOptions::NEAREST);
            cache.texture = Some((tkey, t.clone()));
            t
        }
    };
    // The map is in the coordinates of the canvas (as the lay).
    let view = st.view;
    let (w, h) = (map.cols as f64 * map.cell_mm, map.rows as f64 * map.cell_mm);
    let o = map.origin;
    let corners = [
        Point::new(o.x, o.y),
        Point::new(o.x + w, o.y),
        Point::new(o.x + w, o.y + h),
        Point::new(o.x, o.y + h),
    ];
    let uv = [
        Pos2::new(0.0, 0.0),
        Pos2::new(1.0, 0.0),
        Pos2::new(1.0, 1.0),
        Pos2::new(0.0, 1.0),
    ];
    let mut mesh = Mesh::with_texture(texture.id());
    for (p, uv) in corners.iter().zip(uv) {
        mesh.vertices.push(egui::epaint::Vertex {
            pos: view.to_screen(rect, *p),
            uv,
            color: Color32::WHITE,
        });
    }
    mesh.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
    painter.add(egui::Shape::mesh(mesh));
    let mut lines = Vec::new();
    if let Some(limit) = through {
        let places = map.show_through(limit);
        lines.push(match places.first() {
            None => fill(
                "Fabric shows through: no place over {} %.",
                &[&format!("{:.0}", limit * 100.0)],
            ),
            Some(p) => fill(
                "Fabric shows through at {} places (up to {} %; limit {} %).",
                &[
                    &places.len(),
                    &format!("{:.0}", p.worst * 100.0),
                    &format!("{:.0}", limit * 100.0),
                ],
            ),
        });
    }
    if let Some(limit) = high {
        let places = map.high(limit);
        lines.push(match places.first() {
            None => fill(
                "High thread: no place over {} mm (highest {} mm).",
                &[&format!("{limit:.1}"), &format!("{:.2}", map.max_thread())],
            ),
            Some(p) => fill(
                "Thread higher than {} mm at {} places (up to {} mm).",
                &[&format!("{limit:.1}"), &places.len(), &format!("{:.2}", p.worst)],
            ),
        });
    }
    status(painter, ctx, rect, st, &lines);
}

/// Lines of text at the bottom left of the canvas (right of the ruler),
/// over the line of the density map.
fn status(painter: &egui::Painter, ctx: &egui::Context, rect: Rect, st: &AppState, lines: &[String]) {
    let visuals = ctx.global_style().visuals.clone();
    let left = if st.settings.show_rulers {
        crate::workspace::RULER
    } else {
        0.0
    };
    let rect = Rect::from_min_max(Pos2::new(rect.left() + left, rect.top()), rect.max);
    let mut y = rect.bottom() - if st.show_density { 34.0 } else { 8.0 };
    for text in lines.iter().rev() {
        let galley = painter.layout_no_wrap(text.clone(), FontId::proportional(13.0), visuals.text_color());
        let size = galley.size();
        let pos = Pos2::new(rect.left() + 8.0, y - size.y);
        let bg = Rect::from_min_size(pos, size).expand(4.0);
        painter.rect_filled(bg, 3.0, visuals.extreme_bg_color.gamma_multiply(0.9));
        painter.galley(pos, galley, visuals.text_color());
        y -= size.y + 10.0;
    }
}

/// Options: the limits of the checks of the surface.
pub fn options_ui(ui: &mut egui::Ui, s: &mut crate::workspace::Settings) {
    egui::Grid::new("check_options").num_columns(2).show(ui, |ui| {
        ui.label(tr("Fabric shows through"));
        let mut pct = s.show_through_limit * 100.0;
        if ui
            .add(
                egui::DragValue::new(&mut pct)
                    .range(5.0..=90.0)
                    .speed(1.0)
                    .clamp_existing_to_range(false)
                    .suffix(" %"),
            )
            .on_hover_text(tr(
                "The part of the fabric that can show through a fill or a satin before View > Fabric Shows Through marks the place.",
            ))
            .changed()
        {
            s.show_through_limit = pct / 100.0;
        }
        ui.end_row();
        ui.label(tr("Thread height limit"));
        ui.add(
            egui::DragValue::new(&mut s.thread_height_limit_mm)
                .range(0.3..=5.0)
                .speed(0.05)
                .clamp_existing_to_range(false)
                .suffix(" mm"),
        )
        .on_hover_text(tr(
            "View > High Thread marks the places where the thread is higher than this over the fabric (foam and padding do not count): a risk of thread breaks.",
        ));
        ui.end_row();
    });
}

#[cfg(test)]
mod tests {
    use emb_geom::Path;
    use emb_model::{Design, FillParams, ObjectKind, Rgb};

    use super::*;
    use crate::test_util::Harness;

    /// The exit test of Milestone 8 (the part of the program): a fill whose
    /// rows are too far apart shows the fabric; with the density changed,
    /// the place closes.
    #[test]
    fn a_place_where_the_fabric_shows_closes_with_more_density() {
        let mut h = Harness::empty();
        h.st.stitch_tiles.sync = true;
        let id = h.st.doc.edit("Open fill", |d: &mut Design| {
            let t = d.add_thread("Blue", Rgb(30, 60, 160));
            d.add_object(
                t,
                ObjectKind::Fill {
                    outer: Path::rect(emb_geom::BBox::new(Point::new(-15.0, -15.0), Point::new(15.0, 15.0))),
                    holes: Vec::new(),
                    params: FillParams {
                        spacing_mm: 0.9,
                        underlay: emb_model::Underlay::None,
                        ..FillParams::default()
                    },
                },
            )
        });
        h.st.show_through = true;
        h.frame(Vec::new());
        let (through, _) = h.st.check_places(&h.ctx.clone()).expect("the map");
        assert!(!through.is_empty(), "the fabric shows through the open fill");
        assert!(through[0].worst > 0.3, "{:?}", through[0]);
        // More density: the rows 0.4 mm apart.
        h.st.doc.edit("Density", |d| {
            if let Some(ObjectKind::Fill { params, .. }) = d.object_mut(id).map(|o| &mut o.kind) {
                params.spacing_mm = 0.4;
            }
        });
        h.frame(Vec::new());
        h.frame(Vec::new());
        let (through, _) = h.st.check_places(&h.ctx.clone()).expect("the map");
        assert!(through.is_empty(), "{through:?}");
        h.frame(Vec::new());
    }
}
