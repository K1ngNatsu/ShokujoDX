//! The density map on the canvas (P11 Milestone 1): View > Density map
//! colors the design by the layers of thread (`emb_render::density`), so
//! that the places where objects on top of each other build up show in
//! red. Design info lists the places over the limit (Options).

use std::sync::Arc;

use eframe::egui::{self, Color32, FontId, Mesh, Pos2, Rect, TextureHandle, TextureOptions};
use emb_geom::Point;
use emb_render::density::DensityMap;

use crate::app::AppState;

/// The density map of a plan, and its picture for one limit.
pub struct DensityCache {
    state: u64,
    map: Option<Arc<DensityMap>>,
    texture: Option<(f32, TextureHandle)>,
}

impl AppState {
    /// The density map of the stitch plan that shows (cached).
    pub fn density_map(&mut self) -> Option<Arc<DensityMap>> {
        let pv = self.shown_plan();
        let (state, plan) = (pv.state, pv.plan.clone());
        if let Some(c) = &self.density_cache
            && c.state == state
        {
            return c.map.clone();
        }
        let map = DensityMap::of(&plan.stitches).map(Arc::new);
        self.density_cache = Some(DensityCache {
            state,
            map: map.clone(),
            texture: None,
        });
        map
    }
}

/// The color of a cell: none for little thread, then blue, green and yellow
/// up to the limit, and red over it.
pub fn color(layers: f32, limit: f32) -> Color32 {
    if layers < 0.3 {
        return Color32::TRANSPARENT;
    }
    if layers > limit {
        return Color32::from_rgba_unmultiplied(230, 30, 30, 210);
    }
    const STOPS: [(f32, [u8; 3]); 4] = [
        (0.0, [40, 90, 220]),
        (0.5, [40, 190, 90]),
        (0.8, [240, 210, 40]),
        (1.0, [245, 140, 30]),
    ];
    let t = (layers / limit).clamp(0.0, 1.0);
    let i = STOPS
        .iter()
        .rposition(|(s, _)| *s <= t)
        .unwrap_or(0)
        .min(STOPS.len() - 2);
    let (a, b) = (STOPS[i], STOPS[i + 1]);
    let f = (t - a.0) / (b.0 - a.0);
    let mix = |k: usize| (f32::from(a.1[k]) + (f32::from(b.1[k]) - f32::from(a.1[k])) * f).round() as u8;
    Color32::from_rgba_unmultiplied(mix(0), mix(1), mix(2), 120)
}

/// Options: the density limit and the prices of the thread.
pub fn options_ui(ui: &mut egui::Ui, s: &mut crate::workspace::Settings) {
    use crate::i18n::tr;
    egui::Grid::new("density_options").num_columns(2).show(ui, |ui| {
        ui.label(tr("Density limit"));
        ui.add(
            egui::DragValue::new(&mut s.density_limit)
                .range(2.0..=12.0)
                .speed(0.1)
                .clamp_existing_to_range(false)
                .suffix(tr(" layers")),
        )
        .on_hover_text(tr(LIMIT_HELP));
        ui.end_row();
        let p = &mut s.thread_prices;
        ui.label(tr("Top thread spool"));
        ui.horizontal(|ui| {
            ui.add(egui::DragValue::new(&mut p.spool_m).range(10.0..=50_000.0).suffix(" m"));
            ui.label(tr("price"));
            ui.add(
                egui::DragValue::new(&mut p.spool_price)
                    .range(0.0..=1000.0)
                    .speed(0.05)
                    .max_decimals(2),
            );
        });
        ui.end_row();
        ui.label(tr("Bobbin"));
        ui.horizontal(|ui| {
            ui.add(egui::DragValue::new(&mut p.bobbin_m).range(1.0..=10_000.0).suffix(" m"));
            ui.label(tr("price"));
            ui.add(
                egui::DragValue::new(&mut p.bobbin_price)
                    .range(0.0..=1000.0)
                    .speed(0.01)
                    .max_decimals(2),
            );
        });
        ui.end_row();
        ui.label(tr("Currency"));
        ui.add(egui::TextEdit::singleline(&mut p.currency).desired_width(40.0))
            .on_hover_text(tr(
                "The text before the cost, for example $ or €. A price of 0 gives no cost in Design info.",
            ));
        ui.end_row();
    });
}

/// The tooltip of the density limit.
pub const LIMIT_HELP: &str = "Places with more layers of thread than this show red on the density map and are in the warnings of Design info. A fill with its underlay has about 2 layers, a satin column with its underlay 3 to 4, a satin on a fill 4 to 6.";

/// The picture of the map: one pixel for each cell.
fn image(map: &DensityMap, limit: f32) -> egui::ColorImage {
    let pixels = map.layers.iter().map(|l| color(*l, limit)).collect();
    egui::ColorImage::new([map.cols, map.rows], pixels)
}

/// Draws the map over the stitches, with a line that gives the most layers.
pub fn draw(painter: &egui::Painter, ctx: &egui::Context, rect: Rect, st: &mut AppState) {
    let Some(map) = st.density_map() else {
        return;
    };
    let limit = st.settings.density_limit as f32;
    let Some(cache) = st.density_cache.as_mut() else {
        return;
    };
    let texture = match &cache.texture {
        Some((l, t)) if *l == limit => t.clone(),
        _ => {
            let t = crate::resources::load_texture(ctx, "density-map", image(&map, limit), TextureOptions::LINEAR);
            cache.texture = Some((limit, t.clone()));
            t
        }
    };
    // The map is in plan (machine) coordinates, as the stitches.
    let back = st.doc.design().from_hoop_transform();
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
            pos: view.to_screen(rect, back.apply(*p)),
            uv,
            color: Color32::WHITE,
        });
    }
    mesh.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
    painter.add(egui::Shape::mesh(mesh));
    let max = map.max_layers();
    let text = crate::i18n::fill(
        "Density map: up to {} layers of thread (limit {})",
        &[&format!("{max:.1}"), &format!("{limit:.1}")],
    );
    let visuals = ctx.global_style().visuals.clone();
    let pos = rect.left_bottom() + egui::vec2(8.0, -8.0);
    let galley = painter.layout_no_wrap(text, FontId::proportional(13.0), visuals.text_color());
    let bg = Rect::from_min_size(pos - egui::vec2(0.0, galley.size().y), galley.size()).expand(4.0);
    painter.rect_filled(bg, 3.0, visuals.extreme_bg_color.gamma_multiply(0.9));
    painter.galley(bg.min + egui::vec2(4.0, 4.0), galley, visuals.text_color());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors_go_from_none_to_red_over_the_limit() {
        assert_eq!(color(0.1, 5.0), Color32::TRANSPARENT);
        let low = color(1.0, 5.0);
        let high = color(4.9, 5.0);
        let over = color(5.5, 5.0);
        assert!(low.b() > low.r(), "few layers are blue: {low:?}");
        assert!(high.r() > high.b(), "near the limit is yellow to orange: {high:?}");
        assert!(
            over.to_srgba_unmultiplied()[0] > 200 && over.to_srgba_unmultiplied()[1] < 60 && over.a() > high.a(),
            "over the limit is red: {over:?}"
        );
    }

    #[test]
    fn the_map_of_the_plan_is_cached_and_drawn() {
        let mut h = crate::test_util::Harness::new();
        let a = h.st.density_map().expect("a map");
        let b = h.st.density_map().expect("a map");
        assert!(Arc::ptr_eq(&a, &b));
        h.st.show_density = true;
        h.frame(Vec::new());
    }
}
